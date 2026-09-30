# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "tifffile", "imagecodecs", "japan-geoid==0.6.0"]
# ///
"""Validate a geoid COG against the japan-geoid 0.6 crate and GSI's calculator.

    uv run validate_geoid.py work/out/geoid/jpgeo2024-hrefconv2024/geoid.tif
    uv run validate_geoid.py work/out/geoid/gsigeo2011-v2.2/geoid.tif

The PyPI ``japan-geoid`` 0.6.0 wheel is the same Rust crate the tile server
links (tile/Cargo.toml: japan-geoid 0.6), with the same embedded grids.
"""

from __future__ import annotations

import argparse
import json
import math
import time
import urllib.request

import numpy as np

import sampler

CALC = {
    "jpgeo2024-hrefconv2024": (
        "https://vldb.gsi.go.jp/sokuchi/surveycalc/geoid/calcgh/cgi/geoidcalc.pl",
        "geoidHeight+HeightReferenceConversion",
    ),
    "gsigeo2011-v2.2": (
        "https://vldb.gsi.go.jp/sokuchi/surveycalc/geoid/calcgh2011/cgi/geoidcalc.pl",
        "geoidHeight",
    ),
}

# Spread over the main islands plus islands where Hrefconv2024 is non-zero.
SPOT = [
    ("Tokyo (Shiba)", 35.6581, 139.7414),
    ("Sapporo", 43.0621, 141.3544),
    ("Fukuoka", 33.5902, 130.4017),
    ("Naha", 26.2124, 127.6809),
    ("Hachijo-jima", 33.1100, 139.7900),
    ("Chichi-jima", 27.0940, 142.1920),
    ("Sado", 38.0180, 138.3680),
]


def crate(model: str):
    import japan_geoid as jg

    if model == "jpgeo2024-hrefconv2024":
        return jg.load_embedded_jpgeo2024_hrefconv2024()
    if model == "gsigeo2011-v2.2":
        return jg.load_embedded_gsigeo2011()
    raise SystemExit(model)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("cog")
    ap.add_argument("--n", type=int, default=5000)
    ap.add_argument("--seed", type=int, default=20261001)
    ap.add_argument("--no-calc", action="store_true")
    args = ap.parse_args()

    r = sampler.open_cog(args.cog)
    model = r.metadata["VREF_MODEL"]
    g = crate(model)
    rng = np.random.default_rng(args.seed)

    # Random points inside the valid-node bbox; keep the ones the crate covers
    # until we have --n of them.
    rows, cols = np.nonzero(~np.isnan(r.data))
    lat_hi = r.y0 - (rows.min() + 0.5) * r.dy
    lat_lo = r.y0 - (rows.max() + 0.5) * r.dy
    lon_lo = r.x0 + (cols.min() + 0.5) * r.dx
    lon_hi = r.x0 + (cols.max() + 0.5) * r.dx
    lats, lons, want = [], [], []
    tried = 0
    nan_mismatch = 0
    while len(lats) < args.n:
        la = rng.uniform(lat_lo, lat_hi, 20000)
        lo = rng.uniform(lon_lo, lon_hi, 20000)
        ref = np.asarray(g.get_heights(lo, la), dtype=np.float64)
        got = sampler.sample_many(r, la, lo)
        tried += la.size
        nan_mismatch += int(np.sum(np.isnan(ref) != np.isnan(got)))
        ok = ~np.isnan(ref)
        lats.extend(la[ok])
        lons.extend(lo[ok])
        want.extend(ref[ok])
    lats = np.array(lats[: args.n])
    lons = np.array(lons[: args.n])
    want = np.array(want[: args.n])
    got = sampler.sample_many(r, lats, lons)
    d = got - want
    # Also every valid node exactly.
    node_lat = r.y0 - (rows + 0.5) * r.dy
    node_lon = r.x0 + (cols + 0.5) * r.dx
    node_ref = np.asarray(g.get_heights(node_lon, node_lat), dtype=np.float64)
    node_got = sampler.sample_many(r, node_lat, node_lon)
    nd = node_got - node_ref
    # A node on the edge of the valid area: the crate's own float arithmetic
    # can land an exact-node query at k-1+0.999999999999 and pull in the
    # nodata neighbour (the sampler snaps that away; see sampler.py).
    pad = np.pad(np.isnan(r.data), 1, constant_values=True)
    nb = np.zeros_like(pad)
    for dr in (-1, 0, 1):
        for dc in (-1, 0, 1):
            nb |= np.roll(np.roll(pad, dr, 0), dc, 1)
    next_to_nodata = nb[1:-1, 1:-1][rows, cols]

    out = {
        "cog": args.cog,
        "model": model,
        "random_points": int(args.n),
        "random_tried_in_bbox": int(tried),
        "coverage_disagreements_in_bbox": int(nan_mismatch),
        "random_nan_in_cog_where_crate_valid": int(np.isnan(got).sum()),
        "random_max_abs_diff_m": float(np.nanmax(np.abs(d))),
        "random_rms_diff_m": float(np.sqrt(np.nanmean(d * d))),
        "nodes_checked": int(rows.size),
        "nodes_nan_mismatch": int(np.sum(np.isnan(node_ref) != np.isnan(node_got))),
        "nodes_nan_in_crate_only": int(np.sum(np.isnan(node_ref) & ~np.isnan(node_got))),
        "nodes_nan_in_cog_only": int(np.sum(~np.isnan(node_ref) & np.isnan(node_got))),
        "nodes_nan_in_crate_only_all_next_to_nodata": bool(np.all(next_to_nodata[np.isnan(node_ref) & ~np.isnan(node_got)])),
        "nodes_max_abs_diff_m": float(np.nanmax(np.abs(nd))),
    }

    if not args.no_calc:
        url, key = CALC[model]
        spots = []
        for name, la, lo in SPOT:
            q = f"{url}?outputType=json&latitude={la}&longitude={lo}"
            for attempt in range(6):  # the CGI answers "server busy" under load
                time.sleep(2 + 3 * attempt)
                with urllib.request.urlopen(urllib.request.Request(q, headers={"User-Agent": "Mozilla/5.0"}), timeout=30) as resp:
                    j = json.load(resp)
                if "OutputData" in j:
                    break
            j = j.get("OutputData", j)
            try:
                gsi = float(j[key])
            except (KeyError, TypeError, ValueError):
                gsi = math.nan
            ours = sampler.sample(r, la, lo)
            spots.append({"name": name, "lat": la, "lon": lo, "gsi": gsi, "cog": round(ours, 6), "diff_m": round(ours - gsi, 6) if not math.isnan(gsi) else None, "gsi_raw": j})
        out["gsi_calculator"] = spots
    print(json.dumps(out, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
