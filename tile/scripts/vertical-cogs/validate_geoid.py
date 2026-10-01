"""``vcog.py validate`` for geoid products.

Compares the built COG, through the reference sampler, with the
``japan-geoid`` 0.6.0 crate (the same crate the tile server links; its PyPI
wheel embeds the same grids) at random points in coverage and at every valid
node, and spot-checks GSI's online geoid calculator.
"""

from __future__ import annotations

import json
import math
import time
import urllib.request

import numpy as np

import sampler

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


def validate(cog: str, cfg: dict, n: int = 5000, seed: int = 20261001, calc: bool = True) -> dict:
    """``cfg`` is the product's [product.validate] table."""
    import japan_geoid as jg

    g = getattr(jg, cfg["crate_loader"])()
    r = sampler.open_cog(cog)
    model = r.metadata["VREF_MODEL"]
    rng = np.random.default_rng(seed)

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
    while len(lats) < n:
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
    lats = np.array(lats[: n])
    lons = np.array(lons[: n])
    want = np.array(want[: n])
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
        "cog": cog,
        "model": model,
        "random_points": int(n),
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

    if calc:
        url, key = cfg["calculator"], cfg["calculator_key"]
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
    return out


def verdict(rep: dict, calc: bool, tol_m: float = 1e-4) -> list[str]:
    """Reasons ``rep`` fails (empty = pass).

    With ``calc`` the GSI calculator spot checks are mandatory: a point for
    which the calculator gave no usable value (server busy on every retry,
    unparseable answer) is a failure, not a skip. Only ``calc=False``
    (``--no-calc``) skips them.
    """
    bad = []
    if not rep["random_max_abs_diff_m"] < tol_m:
        bad.append(f"crate: max |diff| {rep['random_max_abs_diff_m']} m at random points")
    if not rep["nodes_max_abs_diff_m"] < tol_m:
        bad.append(f"crate: max |diff| {rep['nodes_max_abs_diff_m']} m at nodes")
    if rep["coverage_disagreements_in_bbox"]:
        bad.append(f"crate: coverage disagrees at {rep['coverage_disagreements_in_bbox']} random points")
    if rep["nodes_nan_in_cog_only"]:
        bad.append(f"{rep['nodes_nan_in_cog_only']} nodes NaN in the COG but valid in the crate")
    if rep["nodes_nan_in_crate_only"] and not rep["nodes_nan_in_crate_only_all_next_to_nodata"]:
        bad.append("nodes NaN in the crate only that are not on the coverage edge")
    if calc:
        spots = rep.get("gsi_calculator") or []
        if not spots:
            bad.append("GSI calculator: no spot checks were run")
        for s in spots:
            if s.get("diff_m") is None:
                bad.append(f"GSI calculator: no value at {s['name']} ({s['lat']}, {s['lon']}): {s.get('gsi_raw')}")
            elif abs(s["diff_m"]) > tol_m:
                bad.append(f"GSI calculator: {s['name']} differs by {s['diff_m']} m")
    return bad
