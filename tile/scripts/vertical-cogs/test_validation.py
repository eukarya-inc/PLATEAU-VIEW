"""Tests for the validation verdicts, the source-file guard and the geoid parsers.

Synthetic inputs only (no network, no GSI downloads, no R2).

    uv run vcog.py test
"""

from __future__ import annotations

import hashlib
import json
import os
import sys

import numpy as np
import pytest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import build  # noqa: E402
import sampler  # noqa: E402
import validate_geoid  # noqa: E402
import validate_oracle  # noqa: E402
import vref  # noqa: E402
from products import Product  # noqa: E402


# --- 1. geoid verdict: a missing calculator value is a failure ----------------
def _geoid_rep(spots):
    return {
        "random_max_abs_diff_m": 2e-6,
        "nodes_max_abs_diff_m": 2e-6,
        "coverage_disagreements_in_bbox": 0,
        "nodes_nan_in_cog_only": 0,
        "nodes_nan_in_crate_only": 3,
        "nodes_nan_in_crate_only_all_next_to_nodata": True,
        **({"gsi_calculator": spots} if spots is not None else {}),
    }


def _spot(diff):
    return {"name": "P", "lat": 35.0, "lon": 139.0, "diff_m": diff, "gsi_raw": {"ExportData": {"ErrMsg": "busy"}}}


def test_geoid_verdict_passes_with_good_calculator_values():
    assert validate_geoid.verdict(_geoid_rep([_spot(1e-5), _spot(-4e-5)]), calc=True) == []


def test_geoid_verdict_fails_when_calculator_gave_no_value():
    bad = validate_geoid.verdict(_geoid_rep([_spot(1e-5), _spot(None)]), calc=True)
    assert len(bad) == 1 and "no value" in bad[0]


def test_geoid_verdict_fails_when_no_spot_checks_ran():
    assert validate_geoid.verdict(_geoid_rep(None), calc=True)
    assert validate_geoid.verdict(_geoid_rep([]), calc=True)


def test_geoid_verdict_only_no_calc_skips_spot_checks():
    assert validate_geoid.verdict(_geoid_rep(None), calc=False) == []
    assert validate_geoid.verdict(_geoid_rep([_spot(None)]), calc=False) == []


def test_geoid_verdict_other_failures():
    assert validate_geoid.verdict(dict(_geoid_rep([_spot(0.0)]), random_max_abs_diff_m=2e-4), calc=True)
    assert validate_geoid.verdict(dict(_geoid_rep([_spot(0.0)]), nodes_nan_in_cog_only=1), calc=True)
    assert validate_geoid.verdict(dict(_geoid_rep([_spot(0.0)]), nodes_nan_in_crate_only_all_next_to_nodata=False), calc=True)


# --- 2. oracle: every JGD2024 tile must have its JGD2011 counterpart ----------
def _tile(name, bounds, z):
    return {"name": name, "bounds": bounds, "z": np.asarray(z, dtype=np.float32)}


@pytest.fixture
def oracle_inputs(monkeypatch, tmp_path):
    # one product grid: constant dH 0.1 over a lattice around mesh 533946
    i0, j0 = vref.mesh6_sw_node(533946)
    nodes = {(i0 + a, j0 + b): 0.1 for a in range(11) for b in range(11)}
    ng = vref.mesh_nodes_to_grid(nodes)
    gt = ng.geotransform()
    r = sampler.from_array(ng.data, gt[0], gt[3], gt[1], -gt[5])
    monkeypatch.setattr(validate_oracle.sampler.DhGsi, "open", classmethod(lambda cls, d: cls(r, r, {999999})))
    lat0, lon0 = i0 * vref.LAT_STEP_DEG, vref.LON_ORIGIN_DEG + j0 * vref.LON_STEP_DEG
    b1 = (lon0, lat0, lon0 + vref.LON_STEP_DEG, lat0 + vref.LAT_STEP_DEG)
    b2 = (lon0 + vref.LON_STEP_DEG, lat0, lon0 + 2 * vref.LON_STEP_DEG, lat0 + vref.LAT_STEP_DEG)
    old_z = np.full((3, 3), 10.0)
    new_z = np.full((3, 3), 10.1)  # 2011 + 0.1
    new = {"meta": {"file_name": "f.zip", "id": 1, "srs": ["fguuid:jgd2024.bl"]},
           "tiles": {validate_oracle.bkey(b1): _tile("T1.xml", b1, new_z), validate_oracle.bkey(b2): _tile("T2.xml", b2, new_z)}}
    state = {"old_tiles": {validate_oracle.bkey(b1): _tile("T1.tif", b1, old_z), validate_oracle.bkey(b2): _tile("T2.tif", b2, old_z)}}
    monkeypatch.setattr(validate_oracle, "load_2024", lambda *a, **k: new)
    monkeypatch.setattr(validate_oracle, "load_2011", lambda *a, **k: {"names": ["FG-GML-5339-46-00-DEM5A-20161001.tif"], "tiles": state["old_tiles"]})
    monkeypatch.setattr(validate_oracle.gsi, "Client", lambda: None)
    return state, str(tmp_path), (b1, b2)


def test_oracle_passes_when_all_tiles_match(oracle_inputs):
    state, work, _ = oracle_inputs
    (e,) = validate_oracle.run("unused", ["533946:DEM5A:20250620"], work, {}, "unused")
    assert e["tiles_2024_without_2011"] == [] and e["residual_m"]["gsi_rule"]["n"] == 18
    assert validate_oracle.verdict(e) == []


def test_oracle_fails_and_names_tiles_without_2011_counterpart(oracle_inputs):
    state, work, (b1, b2) = oracle_inputs
    del state["old_tiles"][validate_oracle.bkey(b2)]
    (e,) = validate_oracle.run("unused", ["533946:DEM5A:20250620"], work, {}, "unused")
    # the matched subset alone would score 100 %
    assert e["residual_m"]["gsi_rule"]["frac_le_0.005"] == 1.0
    bad = validate_oracle.verdict(e)
    assert bad and "T2.xml" in bad[0]


def test_oracle_verdict_other_failures():
    base = {"tiles_2024_without_2011": [], "pixels_valid_2024_only": 0, "pixels_gsi_rule_nan": 0,
            "residual_m": {"gsi_rule": {"n": 10, "frac_le_0.005": 1.0}}}
    assert validate_oracle.verdict(base) == []
    assert validate_oracle.verdict(dict(base, residual_m={"gsi_rule": {"n": 0}}))
    assert validate_oracle.verdict(dict(base, pixels_valid_2024_only=5))
    assert validate_oracle.verdict(dict(base, pixels_gsi_rule_nan=1))
    assert validate_oracle.verdict(dict(base, residual_m={"gsi_rule": {"n": 10, "frac_le_0.005": 0.98}}))


# --- 3. ISG / ASC header strictness -------------------------------------------
ISG_HEAD = {
    "model name": "TEST",
    "data format": "grid",
    "data ordering": "N-to-S, W-to-E",
    "coord type": "geodetic",
    "coord units": "dms",
    "lat min": "35°00'00\"",
    "lat max": "35°02'00\"",
    "lon min": "139°00'00\"",
    "lon max": "139°03'00\"",
    "delta lat": "0°01'00\"",
    "delta lon": "0°01'30\"",
    "nrows": "3",
    "ncols": "3",
    "nodata": "-9999.0000",
    "ISG format": "2.0",
}
ISG_DATA = "1 2 3\n4 5 6\n7 8 -9999.0000\n"


def _write_isg(path, head):
    lines = ["begin_of_head ====="]
    for k, v in head.items():
        sep = "=" if k in ("lat min", "lat max", "lon min", "lon max", "delta lat", "delta lon", "nrows", "ncols", "nodata", "ISG format") else ":"
        lines.append(f"{k:15s}{sep} {v}")
    lines.append("end_of_head =====")
    path.write_text("\n".join(lines) + "\n" + ISG_DATA, encoding="utf-8")
    return str(path)


def test_isg_valid_orientation(tmp_path):
    g = vref.read_isg(_write_isg(tmp_path / "a.isg", ISG_HEAD)).grid
    assert (g.lat_n, g.lon_w) == pytest.approx((35 + 2 / 60, 139.0))
    assert g.data[0, 0] == 1 and g.data[0, 2] == 3 and g.data[2, 0] == 7  # first value = NW node
    assert np.isnan(g.data[2, 2])


@pytest.mark.parametrize("key,value", [
    ("data ordering", "N-to-S, E-to-W"),
    ("data ordering", "S-to-N, W-to-E"),
    ("data ordering", "N-to-S"),
    ("coord type", "projected"),
    ("coord units", "deg"),
    ("data format", "sparse"),
    ("ISG format", "1.0"),
])
def test_isg_refuses_unsupported_layouts(tmp_path, key, value):
    with pytest.raises(ValueError):
        vref.read_isg(_write_isg(tmp_path / "a.isg", dict(ISG_HEAD, **{key: value})))


@pytest.mark.parametrize("missing", ["coord type", "data ordering", "coord units", "nodata"])
def test_isg_refuses_missing_header_fields(tmp_path, missing):
    head = dict(ISG_HEAD)
    del head[missing]
    with pytest.raises(ValueError, match=missing):
        vref.read_isg(_write_isg(tmp_path / "a.isg", head))


def _write_asc(path, head="20.00000 120.00000 0.016667 0.025000 2 3 1 ver2.2", body="1 2 3\n4 5 999.0000\n"):
    path.write_text(head + "\n" + body, encoding="ascii")
    return str(path)


def test_asc_valid_orientation(tmp_path):
    g = vref.read_gsigeo_asc(_write_asc(tmp_path / "a.asc")).grid
    # first row in the file is the southern one (lat0); stored north-up
    assert g.data[1, 0] == 1 and g.data[0, 0] == 4 and np.isnan(g.data[0, 2])
    assert g.lat_n == pytest.approx(20 + 1 / 60)


@pytest.mark.parametrize("head,body", [
    ("20.00000 120.00000 0.016667 0.025000 2 3 1", "1 2 3\n4 5 6\n"),            # 7 header fields
    ("20.00000 120.00000 0.016667 0.030000 2 3 1 v", "1 2 3\n4 5 6\n"),         # other spacing
    ("20.00000 120.00000 0.016667 0.025000 2 3 1 v", "1 2 3\n4 5\n"),           # too few values
    ("20.00000 120.00000 0.016667 0.025000 2 3 1 v", "1 2 3\n4 5 nan\n"),       # non-finite
])
def test_asc_refuses_bad_files(tmp_path, head, body):
    with pytest.raises(ValueError):
        vref.read_gsigeo_asc(_write_asc(tmp_path / "a.asc", head, body))


# --- 4/5. sources are verified against sources.json before parsing ------------
def _geoid_product(tmp_path):
    work = tmp_path / "work"
    src = work / "src" / "g"
    src.mkdir(parents=True)
    path = _write_asc(src / "g.asc")
    data = open(path, "rb").read()
    (src / "sources.json").write_text(json.dumps({"geoid": {"file": "g.asc", "file_sha256": hashlib.sha256(data).hexdigest(), "file_bytes": len(data)}}))
    p = Product({"id": "g", "kind": "geoid", "version": "v1", "key_prefix": "vertical/geoid/g", "format": "gsigeo-asc", "source": {}})
    return p, str(work), path


def test_verified_source_accepts_matching_file(tmp_path):
    p, work, path = _geoid_product(tmp_path)
    meta = json.load(open(os.path.join(p.src_dir(work), "sources.json")))["geoid"]
    assert build.verified_source(p.src_dir(work), meta) == path


@pytest.mark.parametrize("tamper", ["same-size", "append", "remove"])
def test_build_refuses_sources_that_differ_from_sources_json(tmp_path, tamper):
    p, work, path = _geoid_product(tmp_path)
    data = open(path, "rb").read()
    if tamper == "same-size":
        open(path, "wb").write(data.replace(b"4 5", b"4 6"))
    elif tamper == "append":
        open(path, "ab").write(b"\n")
    else:
        os.remove(path)
    with pytest.raises(build.SourceMismatch):
        build.build(p, work)
    assert not os.path.exists(p.out_dir(work))  # nothing was written


def test_height_correction_build_refuses_tampered_par(tmp_path):
    work = tmp_path / "work"
    src = work / "src" / "h"
    src.mkdir(parents=True)
    par = "for PatchJGD(H)       Ver.1.0.0\n" + "\n" * 14 + "MeshCode   dH(m)     0.00000\n53394611  -0.10000   0.00000\n53394612  -0.20000   0.00000\n"
    (src / "t.par").write_bytes(par.encode("cp932"))
    good = par.encode("cp932")
    (src / "sources.json").write_text(json.dumps({"a": {"file": "t.par", "file_sha256": hashlib.sha256(good).hexdigest(), "file_bytes": len(good)}}))
    p = Product({"id": "h", "kind": "height-correction", "version": "v1", "key_prefix": "vertical/h", "from": "jgd2011", "to": "jgd2024",
                 "grid": [{"name": "a", "file": "dh.tif", "content": "test", "source": {"member": "t.par"}}]})
    (src / "t.par").write_bytes(par.replace("-0.20000", "-0.30000").encode("cp932"))
    with pytest.raises(build.SourceMismatch):
        build.build(p, str(work))
