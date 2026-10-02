"""``demcog.py strict`` and ``demcog.py reproduce``: the served stack against its sources.

**strict** -- for every source grid of the chosen primary meshes, read the
served COGs of that primary at full resolution over the public URL, paint them
in the Worker's order (``<mesh>-fill.tif`` under ``<mesh>.tif``) and classify
every pixel that is valid in at least one source product:

* *reproduced* -- the served value equals (|d| <= 0.01 m) one of the products
  valid there; credited to the highest product (stack order) that matches;
* *missing*    -- the served stack has no data there (a dropped grid);
* *wrong*      -- the served value matches none of the products valid there.

A mesh passes when missing = wrong = 0. Additionally *order_deviation* counts
pixels where two products differ and the served value is the lower product's:
the served stack painted the products in another order than the declared one
(reported, not a failure -- see README, "DEM10A/B and DEM5A/B/C overlap").
This is the check that found the DEM10B grids the original batch dropped.

**reproduce** -- download one served COG, rebuild it from the sources with
the same CRS group and role, and compare: bytes (sha256), grid (size,
geotransform, CRS, nodata), every full-resolution pixel and the overview sizes.
Bytes only match with the same GDAL version and options, so pixels are the
criterion. Mismatching pixels are attributed per mesh: "order" if the served
value is another product's value for that pixel, otherwise "other".
"""

from __future__ import annotations

import collections
import concurrent.futures as cf
import hashlib
import json
import os
import time
import urllib.request

import numpy as np

import build
import meshcode
import plan
import publish
import sources

TOL = 0.01
NODATA = -9999.0


# ---------------------------------------------------------------------------
# served stack (from config.json)
# ---------------------------------------------------------------------------
def served_keys(config_url: str, public_base: str) -> list[str]:
    """Keys of every COG layer in the served config, bottom -> top."""
    doc = json.loads(publish.UrllibHttp().get(publish.cache_busted(config_url)))
    base = public_base.rstrip("/") + "/"
    out = []
    for src in doc.get("sources", {}).values():
        for layer in src.get("layers", []):
            u = urllib.request.unquote(layer.get("url", ""))
            if u.startswith(base):
                out.append(u[len(base):])
    return out


def gdal_http_env() -> None:
    os.environ.setdefault("GDAL_HTTP_USERAGENT", publish.USER_AGENT)
    os.environ.setdefault("GDAL_DISABLE_READDIR_ON_OPEN", "EMPTY_DIR")
    os.environ.setdefault("CPL_VSIL_CURL_ALLOWED_EXTENSIONS", ".tif")
    os.environ.setdefault("GDAL_HTTP_MAX_RETRY", "5")
    os.environ.setdefault("GDAL_HTTP_RETRY_DELAY", "2")
    os.environ.setdefault("GDAL_CACHEMAX", "512")


# ---------------------------------------------------------------------------
# windows
# ---------------------------------------------------------------------------
def window_of(transform, mesh: str, shape: tuple[int, int]) -> tuple[int, int]:
    """(row, col) of ``mesh``'s north-west corner in a raster with ``transform``."""
    west, _, _, north = meshcode.bounds(mesh)
    c = (west - transform.c) / transform.a
    r = (north - transform.f) / transform.e
    ci, ri = round(c), round(r)
    if abs(c - ci) > 1e-3 or abs(r - ri) > 1e-3:
        raise ValueError(f"{mesh} is not pixel-aligned with the raster ({r}, {c})")
    return ri, ci


def cut(ds, mesh: str, shape: tuple[int, int]) -> np.ndarray:
    """Read ``shape`` pixels of ``ds`` at ``mesh`` (outside the raster = nodata)."""
    from rasterio.windows import Window

    h, w = shape
    ri, ci = window_of(ds.transform, mesh, shape)
    out = np.full(shape, NODATA, np.float32)
    r0, c0 = max(ri, 0), max(ci, 0)
    r1, c1 = min(ri + h, ds.height), min(ci + w, ds.width)
    if r1 > r0 and c1 > c0:
        a = ds.read(1, window=Window(c0, r0, c1 - c0, r1 - r0))
        nd = ds.nodata
        if nd is not None and nd != NODATA:
            a = np.where(a == nd, NODATA, a)
        out[r0 - ri : r1 - ri, c0 - ci : c1 - ci] = a
    return out


def valid(a: np.ndarray) -> np.ndarray:
    return a > NODATA + 1


# ---------------------------------------------------------------------------
# per-mesh classification (pure; tested)
# ---------------------------------------------------------------------------
def classify(top: np.ndarray, products: list[tuple[str, np.ndarray]]) -> dict:
    """``products`` bottom -> top (stack order). ``top`` = served composite."""
    tv = valid(top)
    any_v = np.zeros(top.shape, bool)
    matched = np.zeros(top.shape, bool)
    credit: dict[str, int] = {}
    # credit the highest matching product
    for name, a in reversed(products):
        av = valid(a)
        any_v |= av
        hit = av & tv & ~matched & (np.abs(top - a) <= TOL)
        credit[name] = int(hit.sum())
        matched |= hit
    deviation = np.zeros(top.shape, bool)
    for i, (_, hi) in enumerate(products):
        for _, lo in products[:i]:
            both = valid(hi) & valid(lo) & (np.abs(hi - lo) > TOL)
            deviation |= both & tv & (np.abs(top - lo) <= TOL) & (np.abs(top - hi) > TOL)
    r = {
        "valid": int(any_v.sum()),
        "reproduced": int((any_v & matched).sum()),
        "missing": int((any_v & ~tv).sum()),
        "wrong": int((any_v & tv & ~matched).sum()),
        "extra": int((tv & ~any_v).sum()),
        "order_deviation": int(deviation.sum()),
        "credit": {k: credit[k] for k, _ in products},
    }
    r["ok"] = r["missing"] == 0 and r["wrong"] == 0
    return r


def composite(layers: list[np.ndarray]) -> np.ndarray:
    """Paint bottom -> top: each valid pixel of a later layer wins."""
    top = np.full(layers[0].shape, NODATA, np.float32) if layers else None
    for a in layers:
        v = valid(a)
        top[v] = a[v]
    return top


# ---------------------------------------------------------------------------
# strict
# ---------------------------------------------------------------------------
def strict_primary(stack, primary: str, tree: sources.SourceTree, public_base: str, served: set[str]) -> dict[str, dict]:
    import rasterio

    keys = [k for k in (stack.key(primary, "fill"), stack.key(primary, "main")) if k in served]
    layers = [(k, rasterio.open("/vsicurl/" + f"{public_base.rstrip('/')}/{k}")) for k in keys]
    by_mesh: dict[str, list[sources.SourceFile]] = collections.defaultdict(list)
    for f in tree.for_primary(stack.products, primary):
        by_mesh[f.grid.mesh].append(f)
    out = {}
    try:
        for mesh, fs in sorted(by_mesh.items()):
            fs.sort(key=lambda f: stack.rank(f.grid.product))
            prods = []
            for f in fs:
                with rasterio.open(f.path) as d:
                    a = d.read(1)
                    epsg = d.crs.to_epsg() if d.crs else None
                    ri, ci = window_of(d.transform, mesh, a.shape)
                    if (ri, ci) != (0, 0):
                        raise ValueError(f"{f.path}: grid origin is not mesh {mesh}")
                prods.append((f.grid.product, a, f, epsg))
            shape = prods[0][1].shape
            painted = [cut(ds, mesh, shape) for _, ds in layers]
            top = composite(painted) if painted else np.full(shape, NODATA, np.float32)
            r = classify(top, [(p, a) for p, a, _, _ in prods])
            r["grids"] = {p: {"name": f.name, "label": meshcode.EPSG_LABEL.get(e, f"EPSG:{e}")} for p, _, f, e in prods}
            r["layers"] = {k: int(valid(a).sum()) for (k, _), a in zip(layers, painted, strict=True)}
            out[mesh] = r
    finally:
        for _, ds in layers:
            ds.close()
    return out


def strict(cfg, stack, tree, primaries: list[str], report_path: str, threads: int = 6, exclude: tuple[str, ...] = ()) -> dict:
    """``exclude``: served keys to leave out (e.g. to see what a -fill COG fixes)."""
    gdal_http_env()
    d = cfg.defaults
    served = set(served_keys(d["config_url"], d["public_base"])) - set(exclude)
    result: dict[str, dict] = {}
    t0 = time.time()
    with cf.ThreadPoolExecutor(threads) as ex:
        futs = {ex.submit(strict_primary, stack, p, tree, d["public_base"], served): p for p in primaries}
        for fut in cf.as_completed(futs):
            p = futs[fut]
            res = fut.result()
            result.update(res)
            bad = [m for m, r in res.items() if not r["ok"]]
            dev = sum(r["order_deviation"] for r in res.values())
            print(f"{stack.id}/{p}: {len(res)} meshes, {len(res) - len(bad)} ok"
                  + (f", FAIL {bad}" if bad else "") + (f", order deviation {dev:,} px" if dev else "")
                  + f"  ({time.time() - t0:.0f}s)", flush=True)
    summary = summarize_strict(result)
    os.makedirs(os.path.dirname(report_path), exist_ok=True)
    with open(report_path, "w") as f:
        json.dump({"stack": stack.id, "primaries": primaries, "summary": summary, "meshes": result}, f, indent=0)
    return summary


def summarize_strict(result: dict[str, dict]) -> dict:
    fail = {m: {k: r[k] for k in ("valid", "missing", "wrong")} for m, r in result.items() if not r["ok"]}
    dev = {m: r["order_deviation"] for m, r in result.items() if r["order_deviation"]}
    multi = [m for m, r in result.items() if len(r["credit"]) > 1]
    return {
        "meshes": len(result), "ok": len(result) - len(fail), "failed": fail,
        "valid_px": sum(r["valid"] for r in result.values()),
        "reproduced_px": sum(r["reproduced"] for r in result.values()),
        "missing_px": sum(r["missing"] for r in result.values()),
        "wrong_px": sum(r["wrong"] for r in result.values()),
        "multi_product_meshes": len(multi),
        "order_deviation_meshes": len(dev), "order_deviation_px": sum(dev.values()),
    }


# ---------------------------------------------------------------------------
# reproduce
# ---------------------------------------------------------------------------
def download(url: str, dst: str) -> tuple[str, str, int]:
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    s, m, n = hashlib.sha256(), hashlib.md5(), 0
    req = urllib.request.Request(url, headers={"User-Agent": publish.USER_AGENT, "Cache-Control": "no-cache"})
    with urllib.request.urlopen(req, timeout=600) as r, open(dst + ".part", "wb") as f:
        for chunk in iter(lambda: r.read(1 << 20), b""):
            f.write(chunk)
            s.update(chunk)
            m.update(chunk)
            n += len(chunk)
    os.replace(dst + ".part", dst)
    return s.hexdigest(), m.hexdigest(), n


def _valid_nd(x: np.ndarray, nodata) -> np.ndarray:
    v = ~np.isnan(x) if x.dtype.kind == "f" else np.ones(x.shape, bool)
    return v & (x != nodata) if nodata is not None else v


def compare_rasters(a_path: str, b_path: str, rows_per_chunk: int = 1024) -> dict:
    """Pixel comparison of two single-band rasters (full resolution + overview sizes).

    A pixel is valid when it is not its own raster's nodata value (so a
    served nodata of 255 and a rebuilt -9999 still compare as "both empty";
    the nodata values themselves are reported in ``grid``)."""
    import rasterio
    from rasterio.windows import Window

    with rasterio.open(a_path) as a, rasterio.open(b_path) as b:
        grid = {
            "size": [[a.width, a.height], [b.width, b.height]],
            "transform_max_abs_diff": max(abs(x - y) for x, y in zip(a.transform, b.transform, strict=True)) if a.shape == b.shape else None,
            "epsg": [a.crs.to_epsg(), b.crs.to_epsg()], "nodata": [a.nodata, b.nodata], "dtype": [a.dtypes[0], b.dtypes[0]],
            "overviews": [a.overviews(1), b.overviews(1)],
        }
        # GDAL versions differ in the last bits of a geotransform (e.g. 2e-9 m on a
        # GoogleMapsCompatible grid): allow a millionth of a pixel.
        tol = 1e-6 * abs(a.transform.a)
        grid["transform_tolerance"] = tol
        same_grid = (a.shape == b.shape and grid["transform_max_abs_diff"] <= tol and grid["epsg"][0] == grid["epsg"][1]
                     and a.dtypes == b.dtypes)
        res = {"grid": grid, "same_grid": same_grid, "same_nodata": a.nodata == b.nodata, "same_overviews": a.overviews(1) == b.overviews(1)}
        if not same_grid:
            return res
        n_eq = n_diff = n_valid_a = n_valid_b = 0
        diff_mask = np.zeros(a.shape, bool)
        for r0 in range(0, a.height, rows_per_chunk):
            w = Window(0, r0, a.width, min(rows_per_chunk, a.height - r0))
            x, y = a.read(1, window=w), b.read(1, window=w)
            vx, vy = _valid_nd(x, a.nodata), _valid_nd(y, b.nodata)
            eq = (vx == vy) & (~vx | (x == y))
            n_eq += int(eq.sum())
            n_diff += int((~eq).sum())
            n_valid_a += int(vx.sum())
            n_valid_b += int(vy.sum())
            diff_mask[r0 : r0 + w.height] = ~eq
        res.update({"pixels": a.width * a.height, "identical_px": n_eq, "different_px": n_diff,
                    "valid_px": [n_valid_a, n_valid_b], "pixel_identical": n_diff == 0})
        res["_diff_mask"] = diff_mask
        res["_transform"] = a.transform
        return res


def attribute_diffs(served_path: str, rebuilt_path: str, diff_mask: np.ndarray, transform, grids: list[dict], tree_root: str, stack) -> dict:
    """Classify differing pixels per mesh.

    * ``served_nodata``  -- the served COG has no data where the rebuild has (a grid the served file lacks)
    * ``rebuilt_nodata`` -- the other way round
    * ``order``          -- both valid; the served value is another product's value for that pixel
                            (the served file painted overlapping products in another order)
    * ``other``          -- anything else
    """
    import rasterio

    by_mesh: dict[str, list[dict]] = collections.defaultdict(list)
    for g in grids:
        by_mesh[g["mesh"]].append(g)
    tot = collections.Counter()
    meshes = {}
    with rasterio.open(served_path) as s, rasterio.open(rebuilt_path) as b:
        for mesh, gs in sorted(by_mesh.items()):
            arrs = []
            for g in gs:
                with rasterio.open(os.path.join(tree_root, g["product"].lower(), g["name"])) as d:
                    arrs.append(d.read(1))
            shape = arrs[0].shape
            ri, ci = window_of(transform, mesh, shape)
            dm = diff_mask[max(ri, 0) : ri + shape[0], max(ci, 0) : ci + shape[1]]
            if dm.shape != shape or not dm.any():
                continue
            sv, bv = cut(s, mesh, shape), cut(b, mesh, shape)
            explained = np.zeros(shape, bool)
            for a in arrs:
                explained |= valid(a) & (np.abs(sv - a) <= TOL)
            c = {
                "different_px": int(dm.sum()),
                "served_nodata": int((dm & ~valid(sv) & valid(bv)).sum()),
                "rebuilt_nodata": int((dm & valid(sv) & ~valid(bv)).sum()),
                "order": int((dm & valid(sv) & valid(bv) & explained).sum()),
                "other": int((dm & valid(sv) & valid(bv) & ~explained).sum()),
                "products": sorted({g["product"] for g in gs}, key=stack.rank),
            }
            meshes[mesh] = c
            tot.update({k: v for k, v in c.items() if k != "products"})
    return {"totals": dict(tot), "meshes": meshes}


def reproduce_patch(cfg, key: str, src_dir: str, work: str, log=print) -> dict:
    """Rebuild one served patch COG from its source directory and compare it."""
    import patch

    d = cfg.defaults
    served_path = os.path.join(work, "served", *key.split("/"))
    sha, md5, n = download(f"{d['public_base'].rstrip('/')}/{urllib.request.quote(key)}", served_path)
    log(f"{key}: served {n:,} B sha256 {sha}")
    out_root = os.path.join(work, "repro")
    m = patch.build_patch(key[len("patch/"):-len(".tif")], src_dir, out_root, force=True, log=log)
    cmp = compare_rasters(served_path, os.path.join(out_root, *key.split("/")))
    cmp.pop("_diff_mask", None), cmp.pop("_transform", None)
    return {"key": key, "served": {"bytes": n, "sha256": sha, "md5": md5},
            "rebuilt": {"bytes": m["bytes"], "sha256": m["sha256"], "gdal": m["gdal"], "inputs": m["inputs"], "crs": m["crs"], "nodata_in": m["nodata_in"]},
            "byte_identical": sha == m["sha256"], **cmp}


def reproduce_key(cfg, key: str, tree, work: str, log=print) -> dict:
    """Rebuild one served base COG from ``tree`` (local grids) and compare it with the served object."""
    import rasterio

    stack, primary, role = cfg.parse_key(key)
    d = cfg.defaults
    url = f"{d['public_base'].rstrip('/')}/{key}"
    served_path = os.path.join(work, "served", *key.split("/"))
    sha, md5, n = download(url, served_path)
    with rasterio.open(served_path) as s:
        epsg = s.crs.to_epsg()
    log(f"{key}: served {n:,} B sha256 {sha} ({meshcode.EPSG_LABEL.get(epsg, epsg)})")
    # the rebuilt group must get the served file's CRS *and* role
    grids = [plan.Grid(f.name, f.grid.mesh, f.grid.product, f.grid.edition, build.inspect_grid(f.path, stack.pixel), os.path.abspath(f.path))
             for f in tree.for_primary(stack.products, primary)]
    epsgs = sorted({g.epsg for g in grids})
    if role == "main":
        pin = epsg
    else:
        others = [e for e in epsgs if e != epsg]
        if len(others) != 1:
            raise RuntimeError(f"{key}: served fill is EPSG:{epsg} but the sources have CRSs {epsgs}")
        pin = others[0]
    out_root = os.path.join(work, "repro")
    ms = build.build_primary(stack, primary, tree, out_root, main_epsg=pin, only_epsg=epsg, force=True, log=log)
    m = ms[0]
    if m["key"] != key:
        raise RuntimeError(f"rebuilt {m['key']}, expected {key}")
    rebuilt = os.path.join(out_root, *key.split("/"))
    cmp = compare_rasters(served_path, rebuilt)
    diff_mask, transform = cmp.pop("_diff_mask", None), cmp.pop("_transform", None)
    res = {"key": key, "served": {"bytes": n, "sha256": sha, "md5": md5, "epsg": epsg},
           "rebuilt": {"bytes": m["bytes"], "sha256": m["sha256"], "gdal": m["gdal"], "grids": len(m["grids"])},
           "byte_identical": sha == m["sha256"], **cmp}
    if diff_mask is not None and diff_mask.any():
        res["attribution"] = attribute_diffs(served_path, rebuilt, diff_mask, transform, m["grids"], tree.root, stack)
    return res
