# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "tifffile", "imagecodecs"]
# ///
"""End-to-end check of the ΔH COG + sampler against GSI's own DEM conversion.

GSI produced the JGD2024 edition of its 5 m / 1 m DEMs (the 2025-07-31
re-issue) as ``round(H_2011 + dH, 2)`` with PatchJGD(H). For a secondary
mesh whose JGD2024 edition is a pure re-issue of the 2011 edition, this
script recomputes ``H_2011 + dH(sampled from our COG at the pixel centre)``
and compares it with GSI's JGD2024 values pixel by pixel.

    GSI_LOGIN_CONF=... uv run validate_oracle.py \\
        --dh-dir work/out/hyokorev-jgd2011-to-jgd2024 --work work/oracle \\
        --listing-dem5a dem5a_list.txt --listing-dem1a dem1a_list.txt \\
        473121:DEM5A:20250620 574037:DEM1A:20250423 ...

* JGD2024 side: the named edition, downloaded from the 基盤地図情報 service
  (login; the zip is parsed in memory and never written to disk; only the
  parsed arrays are cached as .npz under --work).
* JGD2011 side: the per-tertiary GeoTIFFs in the R2 backup
  ``r2:plateau-terrain-ortho-backup/terrain/base_terrain/kibanchizu_dem_20250129/s1_geotiff_raw/<type>/``
  (``rclone --config rclone.r2.conf``). That directory is huge and flat, so
  pass a pre-made listing (``rclone lsf ... > listing``) or the script falls
  back to an ``--include``-filtered lsf (slow).

For each mesh it reports the residual for ``gsi_rule`` (the served form:
:class:`sampler.DhGsi` over dh_bm.tif + dh_tr.tif + tr_meshes.json), for
each grid alone, and optionally for the not-served single-grid diagnostic.
"""

from __future__ import annotations

import argparse
import io
import json
import os
import re
import subprocess
import xml.etree.ElementTree as ET
import zipfile

import numpy as np

import gsi
import sampler

NS = {"gml": "http://www.opengis.net/gml/3.2"}
BACKUP = "r2:plateau-terrain-ortho-backup/terrain/base_terrain/kibanchizu_dem_20250129/s1_geotiff_raw"


# --------------------------------------------------------------------------
# GSI JPGIS-GML DEM
# --------------------------------------------------------------------------
def parse_dem_xml(b: bytes):
    root = ET.fromstring(b)
    env = root.find(".//gml:Envelope", NS)
    srs = env.get("srsName")
    lo = [float(v) for v in env.find("gml:lowerCorner", NS).text.split()]
    hi = [float(v) for v in env.find("gml:upperCorner", NS).text.split()]
    ge = root.find(".//gml:GridEnvelope", NS)
    low = [int(v) for v in ge.find("gml:low", NS).text.split()]
    high = [int(v) for v in ge.find("gml:high", NS).text.split()]
    ncols, nrows = high[0] - low[0] + 1, high[1] - low[1] + 1
    sp = root.find(".//gml:startPoint", NS)
    sx, sy = (int(v) for v in sp.text.split()) if sp is not None else (0, 0)
    tl = root.find(".//gml:tupleList", NS).text
    vals = np.array([float(line.split(",", 1)[1]) for line in tl.strip().splitlines() if "," in line])
    g = np.full(nrows * ncols, np.nan)
    start = sy * ncols + sx
    n = min(len(vals), g.size - start)
    g[start : start + n] = vals[:n]
    g[g <= -9998.0] = np.nan
    # gml:Envelope is the outer boundary of the pixels (lat lon order)
    return srs, g.reshape(nrows, ncols), (lo[1], lo[0], hi[1], hi[0])


def bkey(bounds) -> tuple:
    return tuple(round(v, 6) for v in bounds)


def load_2024(cl: gsi.Client, mesh6: str, typ: str, edition: str, cache: str) -> dict:
    path = os.path.join(cache, f"{mesh6}_{typ}_{edition}.npz")
    if os.path.exists(path):
        z = np.load(path, allow_pickle=True)
        return z["tiles"].item()
    eds = [e for e in cl.dem_editions(mesh6, typ) if e["type_code"] == typ and e["file_update_date"].replace("/", "") == edition]
    if len(eds) != 1:
        raise SystemExit(f"{mesh6} {typ} {edition}: catalogue has {len(eds)} matching files")
    blob = cl.download_file(eds[0]["id"])
    tiles = {}
    srs_seen = set()
    with zipfile.ZipFile(io.BytesIO(blob)) as zf:
        for name in zf.namelist():
            if name.lower().endswith(".xml"):
                srs, g, b = parse_dem_xml(zf.read(name))
                srs_seen.add(srs)
                tiles[bkey(b)] = {"name": os.path.basename(name), "bounds": b, "z": g.astype(np.float32)}
            elif name.lower().endswith(".zip"):  # nested zips (some DEM1A packages)
                with zipfile.ZipFile(io.BytesIO(zf.read(name))) as inner:
                    for n2 in inner.namelist():
                        if n2.lower().endswith(".xml"):
                            srs, g, b = parse_dem_xml(inner.read(n2))
                            srs_seen.add(srs)
                            tiles[bkey(b)] = {"name": os.path.basename(n2), "bounds": b, "z": g.astype(np.float32)}
    del blob  # the archive is never persisted
    meta = {"file_name": eds[0]["file_name"], "id": eds[0]["id"], "srs": sorted(srs_seen)}
    out = {"meta": meta, "tiles": tiles}
    np.savez_compressed(path, tiles=np.array(out, dtype=object))
    return out


# --------------------------------------------------------------------------
# JGD2011 side from the R2 backup
# --------------------------------------------------------------------------
def load_2011(mesh6: str, typ: str, listing: str | None, cache: str, rclone_conf: str) -> dict:
    d = os.path.join(cache, f"{mesh6}_{typ}_2011")
    os.makedirs(d, exist_ok=True)
    pat = re.compile(rf"^FG-GML-{mesh6[:4]}-{mesh6[4:]}-\d\d-.*{typ}-\d{{8}}\.tif$")
    if listing:
        names = [l.strip() for l in open(listing) if pat.match(l.strip())]
    else:
        res = subprocess.run(
            ["rclone", "--config", rclone_conf, "lsf", f"{BACKUP}/{typ.lower()}/", "--include", f"FG-GML-{mesh6[:4]}-{mesh6[4:]}-*"],
            check=True, capture_output=True, text=True,
        )
        names = [l for l in res.stdout.split() if pat.match(l)]
    todo = [n for n in names if not os.path.exists(os.path.join(d, n))]
    if todo:
        lst = os.path.join(d, "_files.txt")
        with open(lst, "w") as f:
            f.write("\n".join(todo) + "\n")
        subprocess.run(
            ["rclone", "--config", rclone_conf, "copy", f"{BACKUP}/{typ.lower()}/", d, "--files-from-raw", lst, "--no-traverse"],
            check=True,
        )
    import tifffile

    tiles = {}
    for n in names:
        with tifffile.TiffFile(os.path.join(d, n)) as tf:
            p = tf.pages[0]
            scale = p.tags[33550].value
            tie = p.tags[33922].value
            nod = float(str(p.tags[42113].value).strip("\x00 ")) if 42113 in p.tags else -9999.0
            z = p.asarray().astype(np.float64)
        z[z == nod] = np.nan
        rows, cols = z.shape
        # these files were written from the same gml:Envelope, so compare
        # bounds at 1e-6 deg (their pixel size carries float noise ~1e-12)
        w, north = tie[3], tie[4]
        b = (w, north - rows * scale[1], w + cols * scale[0], north)
        tiles[bkey(b)] = {"name": n, "bounds": b, "z": z.astype(np.float32)}
    return {"names": names, "tiles": tiles}


# --------------------------------------------------------------------------
def pixel_centres(bounds, shape):
    w, s, e, n = bounds
    rows, cols = shape
    dx = (e - w) / cols
    dy = (n - s) / rows
    lon = w + (np.arange(cols) + 0.5) * dx
    lat = n - (np.arange(rows) + 0.5) * dy
    return np.meshgrid(lat, lon, indexing="ij")


def stats(res: np.ndarray) -> dict:
    a = np.abs(res)
    n = int(a.size)
    if n == 0:
        return {"n": 0}
    return {
        "n": n,
        "frac_le_0.005": round(float((a <= 0.005 + 1e-6).mean()), 6),
        "frac_le_0.010": round(float((a <= 0.010 + 1e-6).mean()), 6),
        "p99_abs": round(float(np.quantile(a, 0.99)), 5),
        "max_abs": round(float(a.max()), 5),
        "mean": round(float(res.mean()), 5),
    }


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("meshes", nargs="+", help="mesh6:TYPE:edition, e.g. 473121:DEM5A:20250620")
    ap.add_argument("--dh-dir", required=True, help="directory with dh_bm.tif, dh_tr.tif, tr_meshes.json")
    ap.add_argument("--diagnostic-merged", help="optional dh_merged_diagnostic.tif to score alongside")
    ap.add_argument("--work", default="work/oracle")
    ap.add_argument("--listing-dem5a")
    ap.add_argument("--listing-dem1a")
    ap.add_argument("--rclone-conf", default="rclone.r2.conf")
    args = ap.parse_args()
    os.makedirs(args.work, exist_ok=True)
    if len(args.meshes) > 6:
        raise SystemExit("at most 6 secondary meshes may be downloaded")

    served = sampler.DhGsi.open(args.dh_dir)
    dh = {"dh_bm": served.bm, "dh_tr": served.tr}
    if args.diagnostic_merged:
        dh["merged_diagnostic"] = sampler.open_cog(args.diagnostic_merged)
    cl = gsi.Client()
    report = []
    for spec in args.meshes:
        mesh6, typ, edition = spec.split(":")
        listing = args.listing_dem5a if typ == "DEM5A" else args.listing_dem1a if typ == "DEM1A" else None
        new = load_2024(cl, mesh6, typ, edition, args.work)
        old = load_2011(mesh6, typ, listing, args.work, args.rclone_conf)
        is_tr = int(mesh6) in served.tr_meshes
        acc = {k: [] for k in ("gsi_rule", *dh)}
        only_old = only_new = dh_nan = matched_tiles = 0
        worst_tiles = []
        for key, t_new in new["tiles"].items():
            t_old = old["tiles"].get(key)
            if t_old is None:
                continue
            if t_old["z"].shape != t_new["z"].shape:
                raise SystemExit(f"{spec} {t_new['name']}: shape {t_new['z'].shape} vs {t_old['z'].shape}")
            matched_tiles += 1
            zo = t_old["z"].astype(np.float64)
            zn = t_new["z"].astype(np.float64)
            vo, vn = ~np.isnan(zo), ~np.isnan(zn)
            only_old += int((vo & ~vn).sum())
            only_new += int((vn & ~vo).sum())
            both = vo & vn
            lat, lon = pixel_centres(t_new["bounds"], zn.shape)
            la, lo = lat[both], lon[both]
            samples = {k: sampler.sample_many(r, la, lo) for k, r in dh.items()}
            samples["gsi_rule"] = served.sample_many(la, lo)
            for k, v in samples.items():
                ok = ~np.isnan(v)
                acc[k].append((zo[both][ok] + v[ok]) - zn[both][ok])
                if k == "gsi_rule":
                    dh_nan += int((~ok).sum())
            r = (zo[both] + samples["gsi_rule"]) - zn[both]
            r = r[~np.isnan(r)]
            if r.size:
                worst_tiles.append((float(np.abs(r).max()), t_new["name"], float((np.abs(r) > 0.005 + 1e-6).mean())))
        worst_tiles.sort(reverse=True)
        entry = {
            "mesh": mesh6,
            "type": typ,
            "edition_2024": edition,
            "file_2024": new["meta"]["file_name"],
            "srs_2024": new["meta"]["srs"],
            "gsi_tr_fallback": is_tr,
            "tiles_2024": len(new["tiles"]),
            "tiles_2011": len(old["tiles"]),
            "tiles_matched": matched_tiles,
            "editions_2011": sorted({re.search(r"-(\d{8})\.tif$", n).group(1) for n in old["names"]}),
            "pixels_valid_2011_only": only_old,
            "pixels_valid_2024_only": only_new,
            "pixels_gsi_rule_nan": dh_nan,
            "residual_m": {k: stats(np.concatenate(v) if v else np.array([])) for k, v in acc.items()},
            "worst_tiles_gsi_rule": worst_tiles[:5],
        }
        report.append(entry)
        print(json.dumps(entry, ensure_ascii=False), flush=True)
    with open(os.path.join(args.work, "oracle_report.json"), "w") as f:
        json.dump(report, f, ensure_ascii=False, indent=2)


if __name__ == "__main__":
    main()
