"""Pixel classification (strict), edition selection (fetch), patch/inventory helpers.

    uv run demcog.py test
"""

from __future__ import annotations

import os
import sys

import numpy as np
import pytest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import fetch  # noqa: E402
import inventory  # noqa: E402
import patch  # noqa: E402
import stacks  # noqa: E402
import verify  # noqa: E402

N = -9999.0


def arr(*v):
    return np.array([v], np.float32)


# --- strict classification ------------------------------------------------------
def test_classify_reproduced_missing_wrong_extra():
    b = arr(1, 2, 3, N, 5)
    top = arr(1, N, 9, 4, 5.004)
    r = verify.classify(top, [("DEM10B", b)])
    assert (r["valid"], r["reproduced"], r["missing"], r["wrong"], r["extra"]) == (4, 2, 1, 1, 1)
    assert not r["ok"]


def test_classify_credits_highest_matching_product_and_order_deviation():
    b = arr(10, 10, 10, N)
    a = arr(20, N, 10, 30)
    # pixel 0: served shows B although A differs -> order deviation; pixel 2: A == B, credited to A
    top = arr(10, 10, 10, 30)
    r = verify.classify(top, [("DEM10B", b), ("DEM10A", a)])
    assert r["ok"] and r["reproduced"] == 4 and r["missing"] == 0
    assert r["credit"] == {"DEM10B": 2, "DEM10A": 2}
    assert r["order_deviation"] == 1


def test_classify_dropped_grid_is_missing():
    # the original batch's failure mode: the grid is not in any served COG
    b = arr(5, 6, 7)
    r = verify.classify(arr(N, N, N), [("DEM10B", b)])
    assert r["missing"] == 3 and not r["ok"]


def test_composite_paints_bottom_to_top():
    fill, main = arr(1, 1, N), arr(N, 2, 2)
    assert verify.composite([fill, main]).tolist() == [[1, 2, 2]]


def test_window_alignment():
    from rasterio.transform import from_origin

    t = from_origin(130.0, 32.0, 1 / 9000, 1 / 9000)
    assert verify.window_of(t, "473067", (750, 1125)) == (750, 7 * 1125)
    with pytest.raises(ValueError):
        verify.window_of(from_origin(130.00001, 32.0, 1 / 9000, 1 / 9000), "473067", (750, 1125))


def test_summary():
    res = {"a": {"ok": True, "valid": 3, "reproduced": 3, "missing": 0, "wrong": 0, "order_deviation": 2, "credit": {"X": 1, "Y": 2}},
           "b": {"ok": False, "valid": 2, "reproduced": 0, "missing": 2, "wrong": 0, "order_deviation": 0, "credit": {"X": 0}}}
    s = verify.summarize_strict(res)
    assert s["failed"] == {"b": {"valid": 2, "missing": 2, "wrong": 0}} and s["order_deviation_meshes"] == 1 and s["multi_product_meshes"] == 1


# --- fetch edition selection --------------------------------------------------------
def rec(mesh, product, date, rid=1, seq=0):
    return {"id": rid, "type_code": product, "place_code": mesh, "file_name": f"FG-GML-{mesh}-{product}-{date.replace('-', '')}.zip",
            "file_update_date": date.replace("-", "/"), "file_split_seq_no": seq}


RECS = [rec("473025", "DEM5A", "2016-10-01", 1), rec("473025", "DEM5A", "2024-12-13", 2), rec("473025", "DEM5A", "2025-06-20", 3),
        rec("473067", "DEM10B", "2009-02-01", 4), rec("473067", "DEM10B", "2016-10-01", 5)]


def test_select_latest_and_before():
    assert sorted(r["id"] for r in fetch.select(RECS)) == [3, 5]
    assert sorted(r["id"] for r in fetch.select(RECS, before="2025-04-01")) == [2, 5]
    assert [r["id"] for r in fetch.select(RECS, before="2010-01-01")] == [4]


def test_select_as_listed():
    got = fetch.select(RECS, as_listed={("DEM5A", "473025"): "2016-10-01", ("DEM10B", "473067"): "2016-10-01"})
    assert sorted(r["id"] for r in got) == [1, 5]
    with pytest.raises(fetch.FetchError, match="offers"):
        fetch.select(RECS, as_listed={("DEM5A", "473025"): "2020-01-01"})
    with pytest.raises(fetch.FetchError, match="no edition"):
        fetch.select(RECS, as_listed={("DEM5B", "473025"): "2016-10-01"})


def test_select_refuses_split_archives():
    with pytest.raises(fetch.FetchError, match="split"):
        fetch.select([rec("473025", "DEM5A", "2016-10-01", seq=1)])


# --- patch / inventory --------------------------------------------------------------
def test_patch_uniformity():
    assert patch.check_uniform({"a": -9999.0, "b": -9999.0}, "NoData") == -9999.0
    with pytest.raises(patch.PatchError, match="NoData MISMATCH"):
        patch.check_uniform({"a": -9999.0, "b": 255.0}, "NoData")
    with pytest.raises(patch.PatchError, match="CRS MISMATCH"):
        patch.check_uniform({"a": "EPSG:6676", "b": "EPSG:6677"}, "CRS")


def test_inventory_summary_flags_mixed_primaries():
    cfg = stacks.load()
    rows = [{"name": "a", "mesh": "473067", "product": "DEM10B", "edition": "2016-10-01", "epsg": 6668},
            {"name": "b", "mesh": "473066", "product": "DEM10B", "edition": "2016-10-01", "epsg": 4612},
            {"name": "c", "mesh": "473066", "product": "DEM10A", "edition": "2016-10-01", "epsg": 4612},
            {"name": "d", "mesh": "533900", "product": "DEM10B", "edition": "2016-10-01", "epsg": None}]
    s = inventory.summarize(rows, cfg)
    assert s["mixed_primaries"] == {"dem10/4730": {"jgd2011": 1, "jgd2000": 2}}
    assert s["labels"]["DEM10B"] == {"jgd2011": 1, "jgd2000": 1, "EPSG:None": 1} and s["unreadable"] == ["d"]


@pytest.mark.parametrize("key", ["patch/../../etc/x.tif", "base/dem10/4730.tif", "patch/x.tif"])
def test_reproduce_patch_refuses_bad_keys(tmp_path, key):
    with pytest.raises((ValueError, Exception)):
        verify.reproduce_patch(stacks.load(), key, str(tmp_path), str(tmp_path))
