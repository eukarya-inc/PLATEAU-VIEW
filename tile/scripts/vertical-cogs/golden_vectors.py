# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "tifffile", "imagecodecs"]
# ///
"""Golden (lon, lat) -> dH vectors for ports of the reference sampler.

    uv run golden_vectors.py <product-dir> <out.json>

<product-dir> holds a published height-correction version (manifest.json,
dh_bm.tif, dh_tr.tif, tr_meshes.json -- e.g. the v1 files the tile server
commits under tile/fixtures/vertical/). The expected values come from
``sampler.DhGsi`` (the normative reference), evaluated point by point with the
scalar ``sample`` path. Each point records its secondary mesh (``mesh6_of``) so
a port can check the selection step separately from the interpolation.

Coordinates and expected values are written twice: as JSON numbers (readable)
and as the IEEE-754 bit patterns (``float.hex``), which a port should parse so
that no decimal round trip can move a point across a mesh boundary or a node.

Coverage (fixed seed, so the file is reproducible):

- listed TR mesh 473121 (interior, nodes, cell edges)
- BM-gap cell 48310309 in 483103, a mesh *not* on the list (per-cell TR fallback)
  and its BM neighbours
- both sides of the 473120 | 473121 boundary, and points exactly on it (which
  belong to 473121, the mesh to the east) and on the 31.5N row (north side)
- areas without parameters (Northern Territories, Iwo-to, Senkaku, Nanatsu-jima)
- random points: uniform over the grid extent, and near valid nodes
"""

from __future__ import annotations

import hashlib
import json
import math
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import sampler  # noqa: E402

SEED = 20261001


def mesh6_bounds(code: int) -> tuple[float, float, float, float]:
    """(south, west, north, east) of a JIS X 0410 secondary mesh."""
    p, q, r, s = code // 10000, (code // 100) % 100, (code // 10) % 10, code % 10
    south = p / 1.5 + r * (5 / 60)
    west = 100 + q + s * (7.5 / 60)
    return south, west, south + 5 / 60, west + 7.5 / 60


def mesh8_bounds(code: int) -> tuple[float, float, float, float]:
    south, west, _, _ = mesh6_bounds(code // 100)
    t, u = (code // 10) % 10, code % 10
    s = south + t * (30 / 3600)
    w = west + u * (45 / 3600)
    return s, w, s + 30 / 3600, w + 45 / 3600


def main(argv: list[str]) -> None:
    if len(argv) != 3:
        raise SystemExit(__doc__.split("Coverage")[0])
    d, out = argv[1], argv[2]
    dh = sampler.DhGsi.open(d)
    rng = np.random.default_rng(SEED)
    pts: list[tuple[float, float, str]] = []

    def add(lon: float, lat: float, label: str) -> None:
        pts.append((float(lon), float(lat), label))

    # 473121 (listed): random interior, nodes, cell edges.
    s, w, n, e = mesh6_bounds(473121)
    for _ in range(200):
        add(rng.uniform(w, e), rng.uniform(s, n), "listed-473121")
    for k in range(0, 11, 2):
        for m in range(0, 11, 2):
            add(w + m * 0.0125, s + k * (1 / 120), "listed-473121-node")
    for _ in range(40):
        add(w + rng.integers(0, 10) * 0.0125, rng.uniform(s, n), "listed-473121-cell-edge")

    # 48310309: BM gap cell in a mesh that is not listed, plus its neighbours.
    s, w, n, e = mesh8_bounds(48310309)
    for _ in range(150):
        add(rng.uniform(w, e), rng.uniform(s, n), "bm-gap-48310309")
    for _ in range(80):
        add(rng.uniform(w - 0.0125, e + 0.0125), rng.uniform(s - 1 / 120, n + 1 / 120), "bm-gap-48310309-around")
    s6, w6, n6, e6 = mesh6_bounds(483103)
    for _ in range(100):
        add(rng.uniform(w6, e6), rng.uniform(s6, n6), "bm-483103")

    # 473120 | 473121 boundary (lon 131.125, lat 31.5 .. 31.58333).
    s, w, n, e = mesh6_bounds(473121)
    for _ in range(100):
        lat = rng.uniform(s, n)
        off = 10 ** rng.uniform(-9, -2)
        add(w - off, lat, "boundary-473120-west")
        add(w + off, lat, "boundary-473121-east")
    for _ in range(40):
        add(w, rng.uniform(s, n), "boundary-exact-lon")
    for k in range(11):
        add(w, s + k * (1 / 120), "boundary-exact-node")
    for _ in range(20):
        add(rng.uniform(w - 0.1, w + 0.1), s, "boundary-exact-lat")
    add(w, s, "boundary-exact-corner")

    # Areas without parameters (NaN expected).
    for label, (lon0, lat0, r) in {
        "noparam-northern-territories": (147.5, 44.9, 0.4),
        "noparam-iwoto": (141.32, 24.78, 0.03),
        "noparam-senkaku": (123.475, 25.745, 0.02),
        "noparam-nanatsujima": (136.92, 37.59, 0.02),
    }.items():
        for _ in range(25):
            add(lon0 + rng.uniform(-r, r), lat0 + rng.uniform(-r, r), label)

    # Random over the grid extent, and near valid nodes (so most are valid).
    bm = dh.primary
    for _ in range(1500):
        add(rng.uniform(bm.x0, bm.x0 + bm.cols * bm.dx), rng.uniform(bm.y0 - bm.rows * bm.dy, bm.y0), "random-extent")
    valid = np.argwhere(~np.isnan(bm.data) | ~np.isnan(dh.listed.data))
    for r, c in valid[rng.choice(len(valid), size=1500, replace=False)]:
        lon = bm.x0 + (c + 0.5) * bm.dx + rng.uniform(-1, 1) * bm.dx
        lat = bm.y0 - (r + 0.5) * bm.dy + rng.uniform(-1, 1) * bm.dy
        add(lon, lat, "random-near-node")

    rows = []
    n_nan = 0
    for lon, lat, label in pts:
        v = dh.sample(lat, lon)
        if math.isnan(v):
            n_nan += 1
        rows.append(
            {
                "label": label,
                "lon": lon,
                "lat": lat,
                "lon_hex": lon.hex(),
                "lat_hex": lat.hex(),
                "mesh6": sampler.mesh6_of(lat, lon),
                "dh": None if math.isnan(v) else v,
                "dh_hex": None if math.isnan(v) else v.hex(),
            }
        )

    def sha(name: str) -> str:
        with open(os.path.join(d, name), "rb") as f:
            return hashlib.sha256(f.read()).hexdigest()

    doc = {
        "generator": "tile/scripts/vertical-cogs/golden_vectors.py (sampler.DhGsi, scalar path)",
        "seed": SEED,
        "product": {k: sha(k) for k in sorted(os.listdir(d)) if k.endswith((".tif", ".json"))},
        "points": len(rows),
        "nan": n_nan,
        "vectors": rows,
    }
    with open(out, "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, separators=(",", ":"))
        f.write("\n")
    print(f"{out}: {len(rows)} points, {n_nan} NaN")


if __name__ == "__main__":
    main(sys.argv)
