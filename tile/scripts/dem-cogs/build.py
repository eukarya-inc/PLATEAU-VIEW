"""``demcog.py build``: per-grid GeoTIFFs -> one COG per primary mesh and CRS, with a manifest.

Recipe (the one the served ``base/`` COGs were made with, from
``convert-cogs.sh`` ``cog_dem_geo``)::

    gdalbuildvrt -srcnodata -9999 -vrtnodata -9999 -input_file_list <ordered list> m.vrt
    gdal_translate -of COG -ot Float32 m.vrt out.tif -co BLOCKSIZE=512 -co RESAMPLING=nearest \
        -co COMPRESS=ZSTD -co PREDICTOR=3 -co NUM_THREADS=ALL_CPUS -co BIGTIFF=YES

i.e. BigTIFF, ZSTD + floating-point predictor, 512 px blocks, Float32, NoData
-9999, the source CRS (EPSG:4612 or 6668, geographic, not reprojected), the
source grid spacing (1/9000 deg for dem10, 1/18000 for dem5, 1/90000 for dem1),
nearest-neighbour overviews halving down to below one block (GDAL's default,
as served). ``BLOCKSIZE=512`` and ``-ot Float32`` are GDAL's defaults here,
written out so the recipe is explicit.

Every output gets ``<key>.manifest.json`` next to it (see README, "Manifest").
"""

from __future__ import annotations

import datetime
import hashlib
import json
import os
import subprocess
import tempfile

import meshcode
import plan
import sources

COG_CREATION = [
    "-co", "BLOCKSIZE=512", "-co", "RESAMPLING=nearest", "-co", "COMPRESS=ZSTD", "-co", "PREDICTOR=3",
    "-co", "NUM_THREADS=ALL_CPUS", "-co", "BIGTIFF=YES",
]
BUILDVRT = ["gdalbuildvrt", "-srcnodata", "-9999", "-vrtnodata", "-9999"]
TRANSLATE = ["gdal_translate", "-of", "COG", "-ot", "Float32"]
# provenance.jsonl fields (from gml2tif / fetch) copied into a grid's manifest entry
ORIGIN_FIELDS = ("zip", "zip_sha256", "zip_bytes", "xml", "xml_sha256", "srs_name", "dev_date", "gsi_id", "gsi_file_update_date")


class BuildError(RuntimeError):
    pass


def gdal_version() -> str:
    return subprocess.run(["gdalinfo", "--version"], capture_output=True, text=True, check=True).stdout.strip()


def tool_revision() -> str | None:
    here = os.path.dirname(os.path.abspath(__file__))
    r = subprocess.run(["git", "-C", here, "rev-parse", "HEAD"], capture_output=True, text=True)
    if r.returncode:
        return None
    dirty = subprocess.run(["git", "-C", here, "status", "--porcelain", "--", "."], capture_output=True, text=True).stdout.strip()
    return r.stdout.strip() + ("+dirty" if dirty else "")


def file_hashes(path: str) -> tuple[str, str]:
    s, m = hashlib.sha256(), hashlib.md5()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            s.update(chunk)
            m.update(chunk)
    return s.hexdigest(), m.hexdigest()


def load_provenance(src_root: str) -> dict[str, dict]:
    """``work/src/provenance.jsonl`` (written by gml2tif): grid file name -> row."""
    p = os.path.join(src_root, "provenance.jsonl")
    out: dict[str, dict] = {}
    if os.path.exists(p):
        for line in open(p):
            r = json.loads(line)
            out[os.path.basename(r["file"])] = r
    return out


def inspect_grid(path: str, pixel) -> int | None:
    """EPSG of a source grid, after checking its type, nodata and spacing."""
    import rasterio

    with rasterio.open(path) as d:
        if d.count != 1 or d.dtypes[0] != "float32":
            raise BuildError(f"{path}: {d.count} band(s) of {d.dtypes[0]}, expected one float32 band")
        if d.nodata != -9999:
            raise BuildError(f"{path}: nodata {d.nodata}, expected -9999")
        px = float(pixel)
        if abs(d.transform.a - px) > 1e-11 or abs(-d.transform.e - px) > 1e-11:
            raise BuildError(f"{path}: pixel {d.transform.a!r} x {-d.transform.e!r}, expected {px!r}")
        return d.crs.to_epsg() if d.crs else None


def _epsg(path: str) -> int | None:
    import rasterio

    with rasterio.open(path) as d:
        return d.crs.to_epsg() if d.crs else None


def check_cog(path: str, epsg: int) -> dict:
    info = json.loads(subprocess.run(["gdalinfo", "-json", path], capture_output=True, text=True, check=True).stdout)
    band = info["bands"][0]
    meta = info.get("metadata", {}).get("IMAGE_STRUCTURE", {})
    got = {
        "layout": meta.get("LAYOUT"), "compression": meta.get("COMPRESSION"), "predictor": meta.get("PREDICTOR"),
        "type": band.get("type"), "block": band.get("block"), "nodata": band.get("noDataValue"),
        "epsg": _epsg(path), "size": info.get("size"),
        "overviews": [o["size"] for o in band.get("overviews", [])],
    }
    want = {"layout": "COG", "compression": "ZSTD", "predictor": "3", "type": "Float32", "block": [512, 512], "nodata": -9999.0, "epsg": epsg}
    bad = {k: (got[k], v) for k, v in want.items() if got[k] != v}
    if bad:
        raise BuildError(f"{path}: COG properties differ from the recipe: {bad}")
    return got


def secondary_summary(rows: list[dict]) -> dict:
    """{mesh6: {product: {"labels": {...}, "editions": [...], "grids": n}}} -- the queryable label table."""
    out: dict[str, dict] = {}
    for r in rows:
        e = out.setdefault(r["mesh"][:6], {}).setdefault(r["product"], {"labels": {}, "editions": [], "grids": 0})
        e["labels"][r["label"]] = e["labels"].get(r["label"], 0) + 1
        if r["edition"] not in e["editions"]:
            e["editions"].append(r["edition"])
        e["grids"] += 1
    for m in out.values():
        for e in m.values():
            e["editions"].sort()
    return dict(sorted(out.items()))


def build_primary(stack, primary: str, tree: sources.SourceTree, out_root: str, *, main_epsg: int | None = None,
                  only_epsg: int | None = None, force: bool = False, log=print) -> list[dict]:
    """Build the COG(s) of one primary mesh. Returns the manifests written."""
    if tree.remote:
        raise BuildError("build reads local grids; use `mirror` (backup) or `fetch` + `gml2tif` (GSI) first")
    files = tree.for_primary(stack.products, primary)
    if not files:
        raise BuildError(f"{stack.id}/{primary}: no input grids under {tree.root}")
    prov = load_provenance(tree.root)
    grids = [plan.Grid(f.name, f.grid.mesh, f.grid.product, f.grid.edition, inspect_grid(f.path, stack.pixel), os.path.abspath(f.path)) for f in files]
    groups = plan.plan_primary(grids, stack.products, main_epsg)
    if len(groups) == 2:
        log(f"  {stack.id}/{primary}: MIXED CRS -> {primary}.tif {groups[0].label} ({len(groups[0].grids)} grids), "
            f"{primary}-fill.tif {groups[1].label} ({len(groups[1].grids)} grids)")
    if only_epsg is not None:
        groups = [g for g in groups if g.epsg == only_epsg]
        if not groups:
            raise BuildError(f"{stack.id}/{primary}: no grids in EPSG:{only_epsg}")
    gv, rev = gdal_version(), tool_revision()
    manifests = []
    for gr in groups:
        key = stack.key(primary, gr.role)
        out = os.path.join(out_root, *key.split("/"))
        if os.path.exists(out) and not force:
            raise BuildError(f"{out} exists (pass --force to rebuild)")
        os.makedirs(os.path.dirname(out), exist_ok=True)
        with tempfile.TemporaryDirectory(dir=os.path.dirname(out)) as td:
            lst, vrt = os.path.join(td, "in.list"), os.path.join(td, "m.vrt")
            paths = [g.path for g in gr.grids]
            with open(lst, "w") as f:
                f.write("".join(p + "\n" for p in paths))
            r = subprocess.run([*BUILDVRT, "-input_file_list", lst, vrt], capture_output=True, text=True)
            if r.returncode:
                raise BuildError(f"{key}: gdalbuildvrt failed: {r.stderr.strip()}")
            warn = plan.buildvrt_warnings(r.stderr)
            if warn:
                raise BuildError(f"{key}: gdalbuildvrt warned (an input may have been dropped) -- refusing: {warn}")
            plan.check_vrt_sources(open(vrt).read(), paths)
            tmp = os.path.join(td, "out.tif")
            subprocess.run([*TRANSLATE, "-q", vrt, tmp, *COG_CREATION], check=True)
            props = check_cog(tmp, gr.epsg)
            os.replace(tmp, out)
        sha, md5 = file_hashes(out)
        rows = []
        for g in gr.grids:
            gs, _ = file_hashes(g.path)
            row = {"name": g.name, "mesh": g.mesh, "product": g.product, "edition": g.edition, "label": g.label,
                   "epsg": g.epsg, "bytes": os.path.getsize(g.path), "sha256": gs}
            p = prov.get(g.name)
            if p:
                if p.get("label") != g.label:
                    raise BuildError(f"{g.name}: GeoTIFF says {g.label}, provenance (srsName) says {p.get('label')}")
                row["origin"] = {k: p[k] for k in ORIGIN_FIELDS if k in p}
            rows.append(row)
        m = {
            "key": key, "stack": stack.id, "primary": primary, "role": gr.role, "epsg": gr.epsg, "label": gr.label,
            "bytes": os.path.getsize(out), "sha256": sha, "md5": md5,
            "cog": props,
            "paint_order": list(stack.products),
            "recipe": {"buildvrt": [*BUILDVRT, "-input_file_list", "<grids, bottom -> top>"], "translate": [*TRANSLATE, "<vrt>", "<out>", *COG_CREATION]},
            "gdal": gv, "tool": {"path": "tile/scripts/dem-cogs", "revision": rev},
            "built_at": datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "secondary_meshes": secondary_summary(rows),
            "grids": rows,
        }
        with open(out + ".manifest.json", "w") as f:
            json.dump(m, f, ensure_ascii=False, indent=1)
        log(f"  {key}: {len(rows)} grids, {gr.label}, {m['bytes']:,} B, sha256 {sha}")
        manifests.append(m)
    return manifests


def labels_from_manifests(paths: list[str]) -> list[dict]:
    """Flatten manifests into rows ``mesh6 product labels editions key``."""
    rows = []
    for p in paths:
        m = json.load(open(p))
        for mesh6, prods in m["secondary_meshes"].items():
            for prod, e in prods.items():
                rows.append({"mesh6": mesh6, "product": prod, "labels": e["labels"], "editions": e["editions"], "grids": e["grids"], "key": m["key"]})
    return sorted(rows, key=lambda r: (r["mesh6"], r["product"], r["key"]))


def epsg_of_label(label: str) -> int:
    return meshcode.LABEL_EPSG[label]
