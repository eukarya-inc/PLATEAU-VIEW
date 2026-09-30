# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "tifffile", "imagecodecs"]
# ///
"""Reference sampler for the vertical-reference COGs (ΔH and geoid grids).

This file is the **normative spec** that the tile server (P2, Rust) and the
Cloudflare Worker must reproduce. It reads the full-resolution image of a COG
with nothing but TIFF tag parsing (no GDAL), so every step that a port has to
re-implement is visible here.

    uv run sampler.py <cog.tif> <lat> <lon> [<lat> <lon> ...]

Semantics
---------
1. Georeferencing. Only the full-resolution IFD (the first one) is sampled;
   overviews exist for viewing and must never be read by a sampler. The file
   is EPSG:6668 (JGD2011 geographic 2D; the lat/lon of a point in JGD2011
   and JGD2024 differ by far less than a node spacing, see README) with
   GTRasterTypeGeoKey = PixelIsArea. From ModelTiepoint (tie at raster (0,0)) and
   ModelPixelScale we get::

       x0 = tie_lon, y0 = tie_lat          # outer NW corner of pixel (0,0)
       dx = scale_x, dy = scale_y          # both positive

   Pixel (row r, col c) has its **centre** at
   ``(lon, lat) = (x0 + (c + 0.5) dx, y0 - (r + 0.5) dy)``. The builders put
   the source grid's nodes exactly on those centres.

2. Fractional pixel-centre coordinates, computed in IEEE float64 exactly in
   this order::

       fx = (lon - x0) / dx - 0.5
       fy = (y0 - lat) / dy - 0.5
       fx = round(fx) if |fx - round(fx)| < 1e-9 else fx     # same for fy
       c0 = floor(fx); tx = fx - c0
       r0 = floor(fy); ty = fy - r0

   The 1e-9-cell snap exists because x0 = (node lon) - dx/2 and dx = 1/80°
   etc. are not exactly representable: without it a query at an exact node
   comes out as k - 1e-12 and drags in the neighbour k-1 with a weight of
   1e-12, which turns a valid node next to a nodata node into NaN. 1e-9 of a
   cell is < 0.01 mm on these grids, so the snap never changes a value by
   anything measurable. (GSI's gsigeome uses a 1e-5 snap for the same reason.)

3. Neighbours and weights. The four candidate neighbours are (r0, c0),
   (r0, c0+1), (r0+1, c0), (r0+1, c0+1) with weights
   (1-tx)(1-ty), tx(1-ty), (1-tx)ty, tx·ty. A neighbour whose weight is
   exactly 0 (tx == 0 for the c0+1 column, ty == 0 for the r0+1 row) is
   **not used**: it is neither read nor bounds-checked. This is what makes a
   point lying exactly on a node row/column valid at the last row/column of
   the grid and next to a nodata node, and it is how GSI's own reference
   interpolators (gsigeome for GSIGEO2011, and the ``japan-geoid`` crate)
   behave.

4. Nodata. The result is NaN if any *used* neighbour lies outside the raster
   (no clamping, no edge extension) or holds nodata (NaN in these COGs; a
   numeric GDAL_NODATA value is also honoured). There is no renormalisation
   over the valid neighbours: a partial stencil would invent a value that
   GSI's interpolation does not produce.

5. Value. ``v = v00(1-tx)(1-ty) + v01·tx(1-ty) + v10(1-tx)ty + v11·tx·ty``
   summed in that order in float64 (unused terms are skipped, not added as 0).

The unit tests in test_sampler.py pin each rule on synthetic grids.
"""

from __future__ import annotations

import math
import sys
from dataclasses import dataclass

import numpy as np

# GeoTIFF tag / key ids
TAG_MODEL_PIXEL_SCALE = 33550
TAG_MODEL_TIEPOINT = 33922
TAG_MODEL_TRANSFORMATION = 34264
TAG_GEO_KEY_DIRECTORY = 34735
TAG_GDAL_NODATA = 42113
KEY_RASTER_TYPE = 1025  # GTRasterTypeGeoKey: 1 = PixelIsArea, 2 = PixelIsPoint


@dataclass
class Raster:
    data: np.ndarray  # (rows, cols) float64, NaN where nodata
    x0: float  # longitude of the outer west edge of column 0
    y0: float  # latitude of the outer north edge of row 0
    dx: float  # > 0
    dy: float  # > 0
    metadata: dict[str, str]

    @property
    def rows(self) -> int:
        return self.data.shape[0]

    @property
    def cols(self) -> int:
        return self.data.shape[1]


def from_array(data, x0: float, y0: float, dx: float, dy: float, nodata: float | None = None) -> Raster:
    a = np.asarray(data, dtype=np.float64).copy()
    if nodata is not None and not math.isnan(nodata):
        a[a == nodata] = np.nan
    return Raster(a, float(x0), float(y0), float(dx), float(dy), {})


def open_cog(path: str) -> Raster:
    import tifffile

    with tifffile.TiffFile(path) as tf:
        page = tf.pages[0]  # full resolution only
        tags = page.tags
        if TAG_MODEL_TRANSFORMATION in tags:
            raise ValueError("ModelTransformation not supported (expected tiepoint + scale)")
        scale = tags[TAG_MODEL_PIXEL_SCALE].value
        tie = tags[TAG_MODEL_TIEPOINT].value
        if len(tie) != 6 or tie[0] != 0 or tie[1] != 0:
            raise ValueError(f"expected a single tiepoint at raster (0,0), got {tie}")
        keys = tags[TAG_GEO_KEY_DIRECTORY].value
        raster_type = 1
        for k in range(4, len(keys), 4):
            if keys[k] == KEY_RASTER_TYPE:
                raster_type = keys[k + 3]
        if raster_type != 1:
            raise ValueError("expected PixelIsArea (GTRasterTypeGeoKey=1)")
        nodata = None
        if TAG_GDAL_NODATA in tags:
            nodata = float(str(tags[TAG_GDAL_NODATA].value).strip("\x00 "))
        arr = page.asarray()
        meta: dict[str, str] = {}
        if 42112 in tags:  # GDAL_METADATA
            import re

            for m in re.finditer(r'<Item name="([^"]+)"[^>]*>(.*?)</Item>', tags[42112].value, re.S):
                meta[m.group(1)] = m.group(2)
    if arr.ndim != 2:
        raise ValueError(f"expected a single band, got shape {arr.shape}")
    r = from_array(arr, tie[3], tie[4], scale[0], scale[1], nodata)
    r.metadata = meta
    return r


SNAP = 1e-9  # cells


def _snap(f: float) -> float:
    k = round(f)
    return float(k) if abs(f - k) < SNAP else f


def sample(r: Raster, lat: float, lon: float) -> float:
    """Sample one point. Returns NaN outside coverage (see module docstring)."""
    fx = (lon - r.x0) / r.dx - 0.5
    fy = (r.y0 - lat) / r.dy - 0.5
    if math.isnan(fx) or math.isnan(fy) or math.isinf(fx) or math.isinf(fy):
        return math.nan
    fx = _snap(fx)
    fy = _snap(fy)
    c0 = math.floor(fx)
    r0 = math.floor(fy)
    tx = fx - c0
    ty = fy - r0
    use_c1 = tx != 0.0
    use_r1 = ty != 0.0

    def val(rr: int, cc: int) -> float | None:
        if rr < 0 or cc < 0 or rr >= r.rows or cc >= r.cols:
            return None
        v = float(r.data[rr, cc])
        return None if math.isnan(v) else v

    v00 = val(r0, c0)
    if v00 is None:
        return math.nan
    acc = v00 * (1.0 - tx) * (1.0 - ty)
    if use_c1:
        v01 = val(r0, c0 + 1)
        if v01 is None:
            return math.nan
        acc += v01 * tx * (1.0 - ty)
    if use_r1:
        v10 = val(r0 + 1, c0)
        if v10 is None:
            return math.nan
        acc += v10 * (1.0 - tx) * ty
    if use_c1 and use_r1:
        v11 = val(r0 + 1, c0 + 1)
        if v11 is None:
            return math.nan
        acc += v11 * tx * ty
    return acc


def sample_many(r: Raster, lat: np.ndarray, lon: np.ndarray) -> np.ndarray:
    """Vectorised equivalent of :func:`sample` (same arithmetic, same order)."""
    lat = np.asarray(lat, dtype=np.float64)
    lon = np.asarray(lon, dtype=np.float64)
    fx = (lon - r.x0) / r.dx - 0.5
    fy = (r.y0 - lat) / r.dy - 0.5
    with np.errstate(invalid="ignore"):
        kx = np.round(fx)
        ky = np.round(fy)
        fx = np.where(np.abs(fx - kx) < SNAP, kx, fx)
        fy = np.where(np.abs(fy - ky) < SNAP, ky, fy)
    c0 = np.floor(fx)
    r0 = np.floor(fy)
    tx = fx - c0
    ty = fy - r0
    c0 = np.nan_to_num(c0, nan=-1, posinf=-1, neginf=-1).astype(np.int64)
    r0 = np.nan_to_num(r0, nan=-1, posinf=-1, neginf=-1).astype(np.int64)
    use_c1 = tx != 0.0
    use_r1 = ty != 0.0

    def fetch(rr, cc, used):
        ok = (rr >= 0) & (cc >= 0) & (rr < r.rows) & (cc < r.cols)
        v = np.full(rr.shape, np.nan)
        v[ok] = r.data[rr[ok], cc[ok]]
        # unused neighbours contribute nothing and cannot invalidate
        return np.where(used, v, 0.0), used & np.isnan(v)

    v00, bad00 = fetch(r0, c0, np.ones_like(use_c1))
    v01, bad01 = fetch(r0, c0 + 1, use_c1)
    v10, bad10 = fetch(r0 + 1, c0, use_r1)
    v11, bad11 = fetch(r0 + 1, c0 + 1, use_c1 & use_r1)
    acc = v00 * (1.0 - tx) * (1.0 - ty)
    acc = np.where(use_c1, acc + v01 * tx * (1.0 - ty), acc)
    acc = np.where(use_r1, acc + v10 * (1.0 - tx) * ty, acc)
    acc = np.where(use_c1 & use_r1, acc + v11 * tx * ty, acc)
    bad = bad00 | bad01 | bad10 | bad11 | ~np.isfinite(fx) | ~np.isfinite(fy)
    return np.where(bad, np.nan, acc)


# ---------------------------------------------------------------------------
# JGD2011 -> JGD2024 height correction: the served two-grid form
# ---------------------------------------------------------------------------
# What GSI's 2025-07 DEM re-issue actually did, established by
# validate_oracle.py (see README "What GSI did"); normative text in the
# product's manifest.json "selection_rule":
#
#   dH(p) = TR(p)            if p lies in a secondary mesh listed in tr_meshes.json
#         = BM(p)            else, if BM(p) is not NaN
#         = TR(p)            else (BM out of range at p: per-cell fallback)
#
# with BM(p)/TR(p) = sample(dh_bm.tif / dh_tr.tif, p). Each point is
# evaluated on ONE grid; nodes are never merged, so the result is
# discontinuous along edges between BM- and TR-evaluated cells (intended).


def mesh6_of(lat: float, lon: float) -> int:
    """JIS X 0410 secondary mesh containing (lat, lon); boundaries go N/E."""
    i5 = math.floor(lat * 12.0)  # 5' rows from the equator
    j75 = math.floor((lon - 100.0) * 8.0)  # 7.5' columns from 100E
    p, r = divmod(i5, 8)
    q, s = divmod(j75, 8)
    return ((p * 100 + q) * 10 + r) * 10 + s


def sample_dh_gsi(bm: Raster, tr: Raster, tr_meshes: set[int], lat: float, lon: float) -> float:
    if mesh6_of(lat, lon) in tr_meshes:
        return sample(tr, lat, lon)
    v = sample(bm, lat, lon)
    return v if not math.isnan(v) else sample(tr, lat, lon)


def sample_dh_gsi_many(bm: Raster, tr: Raster, tr_meshes: set[int], lat, lon) -> np.ndarray:
    lat = np.asarray(lat, dtype=np.float64)
    lon = np.asarray(lon, dtype=np.float64)
    i5 = np.floor(lat * 12.0).astype(np.int64)
    j75 = np.floor((lon - 100.0) * 8.0).astype(np.int64)
    m6 = ((i5 // 8 * 100 + j75 // 8) * 10 + i5 % 8) * 10 + j75 % 8
    in_tr = np.isin(m6, np.fromiter(tr_meshes, dtype=np.int64))
    vb = sample_many(bm, lat, lon)
    vt = sample_many(tr, lat, lon)
    return np.where(in_tr | np.isnan(vb), vt, vb)


def load_tr_meshes(path: str) -> set[int]:
    """Read tr_meshes.json (``{"meshes": [473113, ...], ...}``)."""
    import json

    with open(path, encoding="utf-8") as f:
        doc = json.load(f)
    meshes = doc["meshes"]
    if not meshes or not all(isinstance(m, int) and 100000 <= m <= 999999 for m in meshes):
        raise ValueError(f"{path}: 'meshes' must be a non-empty list of 6-digit secondary mesh codes")
    return set(meshes)


class DhGsi:
    """dh_bm.tif + dh_tr.tif + tr_meshes.json, sampled with GSI's rule.

    :meth:`open` takes a local directory holding the three files.
    """

    def __init__(self, bm: Raster, tr: Raster, tr_meshes: set[int]):
        if (bm.x0, bm.y0, bm.dx, bm.dy, bm.data.shape) != (tr.x0, tr.y0, tr.dx, tr.dy, tr.data.shape):
            raise ValueError("dh_bm and dh_tr must share one grid")
        self.bm, self.tr, self.tr_meshes = bm, tr, tr_meshes

    @classmethod
    def open(cls, directory: str) -> "DhGsi":
        import os

        return cls(
            open_cog(os.path.join(directory, "dh_bm.tif")),
            open_cog(os.path.join(directory, "dh_tr.tif")),
            load_tr_meshes(os.path.join(directory, "tr_meshes.json")),
        )

    def sample(self, lat: float, lon: float) -> float:
        return sample_dh_gsi(self.bm, self.tr, self.tr_meshes, lat, lon)

    def sample_many(self, lat, lon) -> np.ndarray:
        return sample_dh_gsi_many(self.bm, self.tr, self.tr_meshes, lat, lon)


def main(argv: list[str]) -> None:
    if len(argv) < 4 or len(argv) % 2 != 0:
        raise SystemExit(__doc__.split("Semantics")[0])
    r = open_cog(argv[1])
    for i in range(2, len(argv), 2):
        lat, lon = float(argv[i]), float(argv[i + 1])
        print(f"{lat} {lon} {sample(r, lat, lon)!r}")


if __name__ == "__main__":
    main(sys.argv)
