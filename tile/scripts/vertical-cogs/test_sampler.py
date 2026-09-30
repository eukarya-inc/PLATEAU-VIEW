# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "tifffile", "imagecodecs", "pytest"]
# ///
"""Unit tests pinning the sampler semantics on synthetic grids.

    uv run --with pytest --with numpy --with tifffile --with imagecodecs pytest -q test_sampler.py
    # or simply: uv run test_sampler.py
"""

from __future__ import annotations

import math
import os
import shutil
import sys
import tempfile

import numpy as np
import pytest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import sampler  # noqa: E402
import vref  # noqa: E402

NAN = float("nan")

# A 3x4 grid of nodes. Node (r, c) sits at lon = 140 + c*0.5, lat = 36 - r*0.25
# so the raster's outer NW corner is (139.75, 36.125).
DX, DY = 0.5, 0.25
X0, Y0 = 140.0 - DX / 2, 36.0 + DY / 2


def node_lonlat(r: int, c: int) -> tuple[float, float]:
    return 140.0 + c * DX, 36.0 - r * DY


def grid(values) -> sampler.Raster:
    return sampler.from_array(np.array(values, dtype=np.float64), X0, Y0, DX, DY)


BASE = [
    [1.0, 2.0, 4.0, 8.0],
    [16.0, 32.0, 64.0, 128.0],
    [256.0, 512.0, 1024.0, 2048.0],
]


def s(r, lat, lon):
    return sampler.sample(r, lat, lon)


def test_nodes_return_stored_value():
    g = grid(BASE)
    for r in range(3):
        for c in range(4):
            lon, lat = node_lonlat(r, c)
            assert s(g, lat, lon) == BASE[r][c]


def test_bilinear_hand_computed():
    g = grid(BASE)
    # 30% of the way from col 1 to col 2, 60% from row 0 to row 1.
    lon = 140.0 + 1.3 * DX
    lat = 36.0 - 0.6 * DY
    tx, ty = 0.3, 0.6
    want = 2 * (1 - tx) * (1 - ty) + 4 * tx * (1 - ty) + 32 * (1 - tx) * ty + 64 * tx * ty
    assert s(g, lat, lon) == pytest.approx(want, abs=1e-12)


def test_reproduces_plane_exactly():
    # Bilinear interpolation of a plane is the plane itself; any offset in
    # the pixel-centre convention (e.g. treating the origin as a node) breaks this.
    rows, cols = 5, 7
    lon_n = 140.0 + np.arange(cols) * DX
    lat_n = 36.0 - np.arange(rows) * DY
    plane = lambda la, lo: 3.0 + 2.0 * (lo - 140.0) - 5.0 * (la - 36.0)  # noqa: E731
    g = grid(plane(lat_n[:, None], lon_n[None, :]))
    rng = np.random.default_rng(0)
    lo = rng.uniform(lon_n[0], lon_n[-1], 2000)
    la = rng.uniform(lat_n[-1], lat_n[0], 2000)
    got = sampler.sample_many(g, la, lo)
    np.testing.assert_allclose(got, plane(la, lo), atol=1e-12)


def test_any_used_nodata_neighbour_gives_nan():
    vals = [row[:] for row in BASE]
    vals[1][2] = NAN
    g = grid(vals)
    # Inside the cell spanned by rows 0-1, cols 1-2 -> uses (1,2) -> NaN.
    assert math.isnan(s(g, 36.0 - 0.5 * DY, 140.0 + 1.5 * DX))
    # Neighbouring cell rows 0-1, cols 2-3 -> also uses (1,2).
    assert math.isnan(s(g, 36.0 - 0.5 * DY, 140.0 + 2.5 * DX))
    # A cell not touching (1,2) is unaffected.
    assert not math.isnan(s(g, 36.0 - 1.5 * DY, 140.0 + 0.5 * DX))


def test_zero_weight_neighbours_are_ignored():
    vals = [row[:] for row in BASE]
    vals[1][2] = NAN
    g = grid(vals)
    # On node row 0, between cols 1 and 2: row 1 has weight 0 -> valid.
    lon = 140.0 + 1.25 * DX
    assert s(g, 36.0, lon) == pytest.approx(2 * 0.75 + 4 * 0.25)
    # On node column 1, between rows 0 and 1: column 2 has weight 0 -> valid.
    lat = 36.0 - 0.5 * DY
    assert s(g, lat, 140.0 + DX) == pytest.approx(2 * 0.5 + 32 * 0.5)
    # Exactly on a valid node next to the nodata one.
    lon, lat = node_lonlat(1, 1)
    assert s(g, lat, lon) == 32.0


def test_snap_to_node_lines():
    vals = [row[:] for row in BASE]
    vals[1][2] = NAN
    g = grid(vals)
    lon, lat = node_lonlat(1, 1)
    # 1e-12 cell off the node column towards the nodata node: snapped -> valid.
    assert s(g, lat, lon + 1e-12 * DX) == 32.0
    # 1e-6 cell off: a genuine (tiny) weight on the nodata node -> NaN.
    assert math.isnan(s(g, lat, lon + 1e-6 * DX))
    # Snap applies below the node too (fx = k - tiny).
    assert s(g, lat, lon - 1e-12 * DX) == 32.0


def test_last_row_and_column_are_reachable_but_not_beyond():
    g = grid(BASE)
    lon, lat = node_lonlat(2, 3)  # SE-most node
    assert s(g, lat, lon) == 2048.0
    # On the last node column, halfway between rows 1 and 2.
    assert s(g, 36.0 - 1.5 * DY, lon) == pytest.approx((128 + 2048) / 2)
    # Past the last node centre but still inside the raster's outer pixel
    # footprint: no clamping / edge extension -> NaN.
    assert math.isnan(s(g, lat, lon + 0.1 * DX))
    assert math.isnan(s(g, lat - 0.1 * DY, lon))
    # Same on the NW side.
    lon0, lat0 = node_lonlat(0, 0)
    assert math.isnan(s(g, lat0, lon0 - 0.1 * DX))
    assert math.isnan(s(g, lat0 + 0.1 * DY, lon0))
    # Far away.
    assert math.isnan(s(g, 0.0, 0.0))


def test_numeric_nodata_is_honoured():
    vals = np.array(BASE)
    vals[0, 0] = -9999.0
    g = sampler.from_array(vals, X0, Y0, DX, DY, nodata=-9999.0)
    assert math.isnan(s(g, 36.0 - 0.5 * DY, 140.0 + 0.5 * DX))


def test_vectorised_matches_scalar_bitwise():
    vals = np.random.default_rng(1).normal(size=(6, 9))
    vals[2, 3] = np.nan
    vals[5, 8] = np.nan
    g = sampler.from_array(vals, X0, Y0, DX, DY)
    rng = np.random.default_rng(2)
    lon = rng.uniform(X0 - DX, X0 + 10 * DX, 5000)
    lat = rng.uniform(Y0 - 7 * DY, Y0 + DY, 5000)
    # include exact node lines
    lon[:200] = 140.0 + rng.integers(0, 9, 200) * DX
    lat[200:400] = 36.0 - rng.integers(0, 6, 200) * DY
    vec = sampler.sample_many(g, lat, lon)
    sca = np.array([s(g, a, b) for a, b in zip(lat, lon)])
    assert np.array_equal(np.isnan(vec), np.isnan(sca))
    ok = ~np.isnan(sca)
    assert np.array_equal(vec[ok], sca[ok])


def test_mesh_nodes_land_on_pixel_centres():
    # Three nodes of the tertiary-mesh lattice; value = mesh code for checking.
    codes = [53394611, 53394612, 53394621]
    nodes = {vref.mesh8_to_node(c): float(c % 1000) for c in codes}
    ng = vref.mesh_nodes_to_grid(nodes)
    gt = ng.geotransform()
    g = sampler.from_array(ng.data, gt[0], gt[3], gt[1], -gt[5])
    for c in codes:
        i, j = vref.mesh8_to_node(c)
        lat = i * vref.LAT_STEP_DEG
        lon = vref.LON_ORIGIN_DEG + j * vref.LON_STEP_DEG
        assert s(g, lat, lon) == pytest.approx(c % 1000, abs=1e-9)
    # SW corner of 53394611 is 35°40'30" N, 139°45'45" E
    i, j = vref.mesh8_to_node(53394611)
    assert i * 30 == (35 * 3600 + 40 * 60 + 30)
    assert 100 * 3600 + j * 45 == 139 * 3600 + 45 * 60 + 45


def test_mesh_code_round_trip():
    for c in (36224780, 68424546, 47312199, 53394611):
        assert vref.node_to_mesh8(*vref.mesh8_to_node(c)) == c


def test_mesh6_of():
    assert sampler.mesh6_of(35.6581, 139.7414) == 533935
    assert sampler.mesh6_of(31.55, 131.2) == 473121
    # boundaries belong to the mesh to the north / east
    assert sampler.mesh6_of(36.0, 140.0) == 544000
    lat, lon = 36.0 - 1e-9, 140.0 - 1e-9
    assert sampler.mesh6_of(lat, lon) == 533977


def test_gsi_rule_selects_tr_in_listed_meshes_and_falls_back_per_cell():
    # Two 2x2-node lattices over mesh 533946's SW cell and its east neighbour.
    lat0 = vref.mesh6_sw_node(533946)[0] * vref.LAT_STEP_DEG
    lon0 = vref.LON_ORIGIN_DEG + vref.mesh6_sw_node(533946)[1] * vref.LON_STEP_DEG
    dx, dy = vref.LON_STEP_DEG, vref.LAT_STEP_DEG
    x0, y0 = lon0 - dx / 2, lat0 + dy + dy / 2  # 2 node rows, north-up
    bm = sampler.from_array([[1.0, 1.0, NAN], [1.0, 1.0, 1.0]], x0, y0, dx, dy)
    tr = sampler.from_array([[2.0, 2.0, 2.0], [2.0, 2.0, 2.0]], x0, y0, dx, dy)
    lat = lat0 + 0.5 * dy
    # first cell: BM complete -> BM
    assert sampler.sample_dh_gsi(bm, tr, set(), lat, lon0 + 0.5 * dx) == 1.0
    # second cell: BM lacks its NE node -> TR for this cell only
    assert sampler.sample_dh_gsi(bm, tr, set(), lat, lon0 + 1.5 * dx) == 2.0
    # listed secondary mesh -> TR even where BM is complete
    assert sampler.sample_dh_gsi(bm, tr, {533946}, lat, lon0 + 0.5 * dx) == 2.0
    got = sampler.sample_dh_gsi_many(bm, tr, {533946}, [lat, lat], [lon0 + 0.5 * dx, lon0 + 1.5 * dx])
    assert list(got) == [2.0, 2.0]
    got = sampler.sample_dh_gsi_many(bm, tr, set(), [lat, lat], [lon0 + 0.5 * dx, lon0 + 1.5 * dx])
    assert list(got) == [1.0, 2.0]


@pytest.mark.skipif(shutil.which("gdal_translate") is None, reason="needs GDAL CLI")
def test_cog_round_trip_keeps_geotransform_and_values():
    codes = [53394611 + d for d in (0, 1, 2, 10, 11, 12)]
    nodes = {vref.mesh8_to_node(c): (c % 100) / 100.0 for c in codes}
    ng = vref.mesh_nodes_to_grid(nodes)
    with tempfile.TemporaryDirectory() as td:
        out = os.path.join(td, "t.tif")
        vref.write_cog(ng, out, {"TEST": "1"}, "test")
        r = sampler.open_cog(out)
    gt = ng.geotransform()
    assert (r.x0, r.y0, r.dx, r.dy) == (gt[0], gt[3], gt[1], -gt[5])
    for c in codes:
        i, j = vref.mesh8_to_node(c)
        lat = i * vref.LAT_STEP_DEG
        lon = vref.LON_ORIGIN_DEG + j * vref.LON_STEP_DEG
        assert sampler.sample(r, lat, lon) == pytest.approx((c % 100) / 100.0, abs=1e-7)
    assert r.metadata.get("TEST") == "1"


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
