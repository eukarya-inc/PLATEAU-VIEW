"""Planning rules: mesh codes, CRS split, paint order, the dropped-input guard.

    uv run demcog.py test
"""

from __future__ import annotations

import os
import sys

import pytest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import meshcode  # noqa: E402
import plan  # noqa: E402
import stacks  # noqa: E402

DEM10 = ("DEM10B", "DEM10A")
DEM5 = ("DEM5C", "DEM5B", "DEM5A")


def g(mesh, product="DEM10B", epsg=4612, edition="2016-10-01"):
    name = meshcode.grid_name(mesh, product, edition)
    return plan.Grid(name, mesh, product, edition, epsg, "/src/" + name)


# --- mesh codes -------------------------------------------------------------
def test_parse_name_variants():
    a = meshcode.parse_name("FG-GML-4730-67-dem10b-20161001.tif")
    assert (a.mesh, a.product, a.edition, a.primary) == ("473067", "DEM10B", "2016-10-01", "4730")
    b = meshcode.parse_name("FG-GML-4730-25-37-DEM5A-20241213.tif")
    assert (b.mesh, b.secondary, b.product) == ("47302537", "473025", "DEM5A")
    assert meshcode.parse_name("FG-GML-6645-10-dem10B-20161001.tif").product == "DEM10B"
    assert meshcode.grid_name("47302537", "dem5a", "2024-12-13") == "FG-GML-4730-25-37-DEM5A-20241213.tif"
    for bad in ("FG-GML-4730-67-DEM10B.tif", "FG-GML-4730-87-DEM10B-20161001.tif", "x.tif", "FG-GML-4730-67-DEM10B-20161001.zip"):
        with pytest.raises(meshcode.MeshError):
            meshcode.parse_name(bad)


def test_bounds():
    w, s, e, n = meshcode.bounds("4730")
    assert (w, e) == (130.0, 131.0) and s == pytest.approx(31 + 1 / 3) and n == pytest.approx(32.0)
    w, s, e, n = meshcode.bounds("473067")  # row 6, column 7
    assert w == pytest.approx(130.875) and s == pytest.approx(31 + 1 / 3 + 6 / 12) and n - s == pytest.approx(1 / 12)
    w, s, e, n = meshcode.bounds("47302537")
    assert e - w == pytest.approx(1 / 80) and n - s == pytest.approx(1 / 120)


def test_labels():
    assert meshcode.label_of_srs("fguuid:jgd2000.bl") == "jgd2000"
    assert meshcode.label_of_epsg(6668) == "jgd2011"
    with pytest.raises(meshcode.MeshError):
        meshcode.label_of_epsg(4326)
    with pytest.raises(meshcode.MeshError):
        meshcode.label_of_srs("fguuid:tokyo.bl")


# --- order ------------------------------------------------------------------
def test_order_is_product_then_mesh():
    gs = [g("473067", "DEM10A"), g("473070"), g("473067"), g("473066", "DEM10A")]
    assert [(x.product, x.mesh) for x in plan.order(gs, DEM10)] == [
        ("DEM10B", "473067"), ("DEM10B", "473070"), ("DEM10A", "473066"), ("DEM10A", "473067")]
    gs = [g("47300000", p, 6668) for p in ("DEM5A", "DEM5C", "DEM5B")]
    assert [x.product for x in plan.order(gs, DEM5)] == ["DEM5C", "DEM5B", "DEM5A"]  # 5A painted last = on top
    with pytest.raises(plan.PlanError):
        plan.order([g("473067", "DEM5A")], DEM10)


def test_unique():
    with pytest.raises(plan.PlanError, match="two grids"):
        plan.check_unique([g("473067", edition="2009-02-01"), g("473067")])


# --- CRS split ----------------------------------------------------------------
def test_split_majority_and_fill():
    gs = [g(f"47300{i}", epsg=4612) for i in range(3)] + [g("473067", epsg=6668)]
    out = plan.split_by_crs(gs)
    assert [(r, e, len(x)) for r, e, x in out] == [("main", 4612, 3), ("fill", 6668, 1)]


def test_split_tie_prefers_newer_datum():
    gs = [g("664500", epsg=4612), g("664501", epsg=6668)]
    assert plan.split_by_crs(gs)[0][:2] == ("main", 6668)


def test_split_pin():
    gs = [g(f"36230{i}", epsg=4612) for i in range(5)] + [g("362300", epsg=6668)]
    assert plan.split_by_crs(gs, main_epsg=6668)[0][:2] == ("main", 6668)
    with pytest.raises(plan.PlanError, match="pinned"):
        plan.split_by_crs([g("362300", epsg=4612)], main_epsg=6668)


def test_split_refuses_unknown_and_three():
    with pytest.raises(plan.PlanError, match="refusing to guess"):
        plan.split_by_crs([g("473067", epsg=None)])
    with pytest.raises(plan.PlanError, match="refusing to guess"):
        plan.split_by_crs([g("473067", epsg=4326)])
    meshcode.EPSG_LABEL[9999] = "test"
    meshcode.LABEL_RANK["test"] = 2
    try:
        with pytest.raises(plan.PlanError, match="3 CRSs"):
            plan.split_by_crs([g("473067", epsg=4612), g("473066", epsg=6668), g("473065", epsg=9999)])
    finally:
        del meshcode.EPSG_LABEL[9999], meshcode.LABEL_RANK["test"]


# --- cross-COG layering -------------------------------------------------------
def test_layering_ok_when_better_product_in_main():
    # 4730 as served: DEM10A 473067 (jgd2000) in the main COG, DEM10B 473067 (jgd2011) in the fill
    gs = [g("473067", "DEM10A", 4612), g("473066", "DEM10B", 4612), g("473065", "DEM10B", 4612), g("473067", "DEM10B", 6668)]
    groups = plan.plan_primary(gs, DEM10)
    assert [(x.role, x.label) for x in groups] == [("main", "jgd2000"), ("fill", "jgd2011")]
    assert [x.product for x in groups[0].grids] == ["DEM10B", "DEM10B", "DEM10A"]


def test_layering_refuses_better_product_in_fill():
    gs = [g("473067", "DEM10A", 6668), g("473066", "DEM10B", 4612), g("473065", "DEM10B", 4612), g("473067", "DEM10B", 4612)]
    with pytest.raises(plan.PlanError, match="painted under"):
        plan.plan_primary(gs, DEM10)


# --- the dropped-input guard ----------------------------------------------------
VRT = """<VRTDataset><VRTRasterBand>
<ComplexSource><SourceFilename relativeToVRT="0">/src/a.tif</SourceFilename></ComplexSource>
<ComplexSource><SourceFilename relativeToVRT="0">/src/b.tif</SourceFilename></ComplexSource>
</VRTRasterBand></VRTDataset>"""


def test_vrt_source_count_and_order():
    plan.check_vrt_sources(VRT, ["/src/a.tif", "/src/b.tif"])
    with pytest.raises(plan.PlanError, match="dropped"):
        plan.check_vrt_sources(VRT, ["/src/a.tif", "/src/b.tif", "/src/c.tif"])
    with pytest.raises(plan.PlanError, match="order"):
        plan.check_vrt_sources(VRT, ["/src/b.tif", "/src/a.tif"])


def test_buildvrt_warnings():
    err = "Warning 6: gdalbuildvrt does not support heterogeneous projection: expected JGD2000, got JGD2011. Skipping x.tif\n"
    assert plan.buildvrt_warnings(err)
    assert plan.buildvrt_warnings("0...10...20...30...40...50...60...70...80...90...100 - done.\n") == []


def test_main_pins():
    assert plan.parse_main_pins("3623=6668, 6545=EPSG:6668") == {"3623": 6668, "6545": 6668}
    assert plan.parse_main_pins(None) == {}
    with pytest.raises(plan.PlanError):
        plan.parse_main_pins("36=6668")


# --- stacks.toml ----------------------------------------------------------------
def test_stacks_file():
    cfg = stacks.load()
    assert cfg.stack("dem10").products == DEM10 and cfg.stack("dem5").products == DEM5
    s, m, r = cfg.parse_key("base/dem10/4730-fill.tif")
    assert (s.id, m, r) == ("dem10", "4730", "fill")
    assert s.key("4730", "main") == "base/dem10/4730.tif"
    assert float(cfg.stack("dem10").pixel) == pytest.approx(1 / 9000)
    for bad in ("base/dem10/473.tif", "patch/x.tif", "base/dem7/4730.tif"):
        with pytest.raises(stacks.StackError):
            cfg.parse_key(bad)


def test_served_pins_are_valid():
    import json

    here = os.path.dirname(os.path.abspath(__file__))
    pins = json.load(open(os.path.join(here, "served", "state.json")))["main_crs"]
    for sid, d in pins.items():
        stacks.load().stack(sid)
        for mesh, epsg in d.items():
            assert len(mesh) == 4 and epsg in meshcode.EPSG_LABEL
