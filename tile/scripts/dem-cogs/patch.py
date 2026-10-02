"""``demcog.py patch``: local-government DEM tiles -> one ``patch/<name>.tif`` COG.

Port of ``convert-cogs.sh`` ``_patch_region`` (the recipe the served
``patch/`` COGs were made with), with its checks made explicit:

* inputs: every ``*.tif`` and ``*.asc`` (ESRI ASCII grid with a ``.prj``)
  under the directory; anything else (e.g. a ``.zip`` next to them) is ignored;
* the NoData value must be the same in every input (read from all of them,
  not from the first one) -- refused otherwise; it is mapped to -9999 in the
  output so a sentinel like 255 cannot collide with real heights;
* the CRS must be the same in every input (EPSG code, or a hash of the PROJ
  string for WKT without an EPSG code, e.g. kisarazu's transverse Mercator
  .prj) -- refused otherwise, because ``gdalbuildvrt`` would silently drop
  the odd ones; and the VRT source count is checked;
* geographic sources (EPSG 4326, 6668, 4612, 4301) keep their CRS and grid
  (same options as ``base/``); projected ones (plane rectangular CS etc.) are
  warped to EPSG:3857 ``GoogleMapsCompatible`` with 256 px blocks; both with
  nearest resampling, ZSTD and PREDICTOR=3, BigTIFF.
"""

from __future__ import annotations

import collections
import hashlib
import json
import os
import subprocess
import tempfile

import build
import plan

GEOGRAPHIC = {4326, 6668, 4612, 4301}
GEO_OPTS = build.COG_CREATION
MERC_OPTS = ["-co", "TILING_SCHEME=GoogleMapsCompatible", "-co", "BLOCKSIZE=256", "-co", "ZOOM_LEVEL_STRATEGY=AUTO",
             "-co", "RESAMPLING=nearest", "-co", "COMPRESS=ZSTD", "-co", "PREDICTOR=3", "-co", "NUM_THREADS=ALL_CPUS", "-co", "BIGTIFF=YES"]


class PatchError(RuntimeError):
    pass


def list_inputs(src_dir: str) -> list[str]:
    out = []
    for root, _, names in os.walk(src_dir):
        for n in names:
            if n.lower().endswith((".tif", ".tiff", ".asc")):
                out.append(os.path.join(root, n))
    return sorted(out)


def crs_key(path: str) -> str:
    """``EPSG:<n>`` or ``wkt<md5[:8]>`` of a raster's CRS; raises if it has none."""
    r = subprocess.run(["gdalsrsinfo", "-o", "epsg", path], capture_output=True, text=True)
    codes = [w.split(":")[1] for w in r.stdout.split() if w.startswith("EPSG:")]
    if codes:
        return f"EPSG:{codes[-1]}"
    p = subprocess.run(["gdalsrsinfo", "-o", "proj4", path], capture_output=True, text=True).stdout
    p = "".join(p.replace("'", "").split())
    if not p:
        raise PatchError(f"{path}: no CRS -- refusing")
    return "wkt" + hashlib.md5(p.encode()).hexdigest()[:8]


def nodata_of(path: str) -> float | None:
    info = json.loads(subprocess.run(["gdalinfo", "-json", path], capture_output=True, text=True, check=True).stdout)
    return info["bands"][0].get("noDataValue")


def check_uniform(values: dict[str, object], what: str) -> object:
    """The single value shared by every input, or PatchError naming the groups (pure; tested)."""
    groups = collections.Counter(map(str, values.values()))
    if len(groups) > 1:
        raise PatchError(f"{what} MISMATCH across inputs: {dict(groups)} -- refusing")
    return next(iter(values.values())) if values else None


def build_patch(name: str, src_dir: str, out_root: str, force: bool = False, log=print) -> dict:
    key = f"patch/{name}.tif"
    files = list_inputs(src_dir)
    if not files:
        raise PatchError(f"{src_dir}: no .tif/.asc inputs")
    nodata = check_uniform({f: nodata_of(f) for f in files}, "NoData")
    crs = check_uniform({f: crs_key(f) for f in files}, "CRS")
    epsg = int(crs.split(":")[1]) if crs.startswith("EPSG:") else None
    geographic = epsg in GEOGRAPHIC
    out = os.path.join(out_root, *key.split("/"))
    if os.path.exists(out) and not force:
        raise PatchError(f"{out} exists (pass --force)")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    ndopt = ["-srcnodata", str(nodata), "-vrtnodata", "-9999"] if nodata is not None else []
    with tempfile.TemporaryDirectory(dir=os.path.dirname(out)) as td:
        lst, vrt = os.path.join(td, "in.list"), os.path.join(td, "m.vrt")
        open(lst, "w").write("".join(f + "\n" for f in files))
        r = subprocess.run(["gdalbuildvrt", *ndopt, "-input_file_list", lst, vrt], capture_output=True, text=True)
        if r.returncode:
            raise PatchError(f"{key}: gdalbuildvrt failed: {r.stderr.strip()}")
        warn = plan.buildvrt_warnings(r.stderr)
        if warn:
            raise PatchError(f"{key}: gdalbuildvrt warned -- refusing: {warn}")
        plan.check_vrt_sources(open(vrt).read(), files)
        tmp = os.path.join(td, "out.tif")
        subprocess.run(["gdal_translate", "-q", "-of", "COG", vrt, tmp, *(GEO_OPTS if geographic else MERC_OPTS)], check=True)
        os.replace(tmp, out)
    sha, md5 = build.file_hashes(out)
    m = {"key": key, "kind": "patch", "src_dir": src_dir, "inputs": len(files), "crs": crs, "nodata_in": nodata,
         "output_grid": "source CRS" if geographic else "EPSG:3857 GoogleMapsCompatible",
         "bytes": os.path.getsize(out), "sha256": sha, "md5": md5, "gdal": build.gdal_version(),
         "tool": {"path": "tile/scripts/dem-cogs", "revision": build.tool_revision()},
         "files": [{"name": os.path.relpath(f, src_dir), "bytes": os.path.getsize(f), "sha256": build.file_hashes(f)[0]} for f in files]}
    with open(out + ".manifest.json", "w") as f:
        json.dump(m, f, ensure_ascii=False, indent=1)
    log(f"  {key}: {len(files)} inputs, {crs}, nodata {nodata} -> {m['output_grid']}, {m['bytes']:,} B")
    return m
