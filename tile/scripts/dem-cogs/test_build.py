"""FG-GML parsing, gml2tif provenance, and an end-to-end build with GDAL.

The build test writes three synthetic DEM10 grids (two labels, one DEM10A
over DEM10B overlap) and checks the CRS split, the paint order, the COG
recipe and the manifest's label table. It needs the GDAL CLI.

    uv run demcog.py test
"""

from __future__ import annotations

import json
import os
import shutil
import sys
import zipfile

import numpy as np
import pytest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import build  # noqa: E402
import fggml  # noqa: E402
import meshcode  # noqa: E402
import plan  # noqa: E402
import sources  # noqa: E402
import stacks  # noqa: E402

GDAL = shutil.which("gdal_translate") is not None


def xml(mesh="473067", srs="fguuid:jgd2011.bl", shape=(1125, 750), start=None, values=None, order="+x-y"):
    w, s, e, n = meshcode.bounds(mesh)
    tuples = "\n".join(f"地表面,{v:.2f}" for v in (values or [])) + "\n"
    sp = f"<gml:startPoint>{start[0]} {start[1]}</gml:startPoint>" if start else ""
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<Dataset xmlns="http://fgd.gsi.go.jp/spec/2008/FGD_GMLSchema" xmlns:gml="http://www.opengis.net/gml/3.2">
<DEM gml:id="DEM001"><mesh>{mesh}</mesh><devDate><gml:timePosition>2016-10-01</gml:timePosition></devDate>
<coverage><gml:boundedBy><gml:Envelope srsName="{srs}"><gml:lowerCorner>{s} {w}</gml:lowerCorner><gml:upperCorner>{n} {e}</gml:upperCorner></gml:Envelope></gml:boundedBy>
<gml:gridDomain><gml:Grid><gml:limits><gml:GridEnvelope><gml:low>0 0</gml:low><gml:high>{shape[0] - 1} {shape[1] - 1}</gml:high></gml:GridEnvelope></gml:limits></gml:Grid></gml:gridDomain>
<gml:rangeSet><gml:DataBlock><gml:tupleList>
{tuples}</gml:tupleList></gml:DataBlock></gml:rangeSet>
<gml:coverageFunction><gml:GridFunction><gml:sequenceRule order="{order}">Linear</gml:sequenceRule>{sp}</gml:GridFunction></gml:coverageFunction>
</coverage></DEM></Dataset>""".encode()


# --- FG-GML -------------------------------------------------------------------
def test_parse_start_point_and_short_tuple_list():
    gr = fggml.parse(xml(start=(3, 1), values=[10.0, -9999.0, 12.5]), "DEM10B")
    assert gr.data.shape == (750, 1125) and gr.label == "jgd2011"
    flat = gr.data.ravel()
    i = 1 * 1125 + 3
    assert flat[i] == np.float32(10.0) and flat[i + 1] == -9999 and flat[i + 2] == np.float32(12.5)
    assert (flat[:i] == -9999).all() and (flat[i + 3:] == -9999).all()
    assert gr.valid_px == 2 and gr.kinds == {"地表面": 3}


def test_parse_refuses_bad_input():
    with pytest.raises(fggml.GmlError, match="sequenceRule"):
        fggml.parse(xml(order="+y-x", values=[1.0]))
    with pytest.raises(fggml.GmlError, match="expected"):
        fggml.parse(xml(shape=(225, 150), values=[1.0]), "DEM10B")
    with pytest.raises(meshcode.MeshError):
        fggml.parse(xml(srs="fguuid:tokyo.bl", values=[1.0]))
    with pytest.raises(fggml.GmlError, match="overflow"):
        fggml.parse(xml(shape=(225, 150), start=(224, 149), values=[1.0, 2.0]), "DEM5A")
    bad_env = xml(values=[1.0]).replace(b"<gml:lowerCorner>", b"<gml:lowerCorner>0.1")
    with pytest.raises((fggml.GmlError, ValueError)):
        fggml.parse(bad_env)


def test_jgd2024_grids_are_refused(tmp_path):
    gr = fggml.parse(xml(srs="fguuid:jgd2024.bl", values=[1.0]), "DEM10B")
    assert gr.label == "jgd2024"
    with pytest.raises(fggml.GmlError, match="not accepted"):
        fggml.write_geotiff(gr, str(tmp_path / "x.tif"))


def test_zip_to_tifs_records_provenance(tmp_path):
    z = tmp_path / "FG-GML-473067-DEM10B-20161001.zip"
    with zipfile.ZipFile(z, "w") as zf:
        zf.writestr("FG-GML-4730-67-DEM10B-20161001.xml", xml(values=[5.0] * 10))
    rows = fggml.zip_to_tifs(str(z), str(tmp_path / "src"))
    assert len(rows) == 1
    r = rows[0]
    assert r["file"] == "dem10b/FG-GML-4730-67-DEM10B-20161001.tif" and r["label"] == "jgd2011" and r["srs_name"] == "fguuid:jgd2011.bl"
    assert r["valid_px"] == 10 and len(r["zip_sha256"]) == 64 and len(r["xml_sha256"]) == 64
    import rasterio

    with rasterio.open(tmp_path / "src" / r["file"]) as d:
        assert d.crs.to_epsg() == 6668 and d.nodata == -9999 and d.shape == (750, 1125)
        assert d.transform.c == pytest.approx(130.875) and d.transform.a == pytest.approx(1 / 9000)


def test_zip_mesh_must_match_name(tmp_path):
    z = tmp_path / "a.zip"
    with zipfile.ZipFile(z, "w") as zf:
        zf.writestr("FG-GML-4730-66-DEM10B-20161001.xml", xml(mesh="473067", values=[1.0]))
    with pytest.raises(fggml.GmlError, match="!= file name mesh"):
        fggml.zip_to_tifs(str(z), str(tmp_path / "src"))


# --- end-to-end build ----------------------------------------------------------
def _grid(tmp, mesh, product, srs, value, mask=None):
    gr = fggml.parse(xml(mesh=mesh, srs=srs, values=[0.0]), product)
    gr.data[:] = value
    if mask is not None:
        gr.data[~mask] = -9999
    d = tmp / "src" / product.lower()
    d.mkdir(parents=True, exist_ok=True)
    fggml.write_geotiff(gr, str(d / meshcode.grid_name(mesh, product, "2016-10-01")))


@pytest.mark.skipif(not GDAL, reason="needs the GDAL CLI")
def test_build_primary_split_order_manifest(tmp_path):
    import rasterio

    half = np.zeros((750, 1125), bool)
    half[:, :500] = True
    _grid(tmp_path, "473066", "DEM10B", "fguuid:jgd2000.bl", 100.0)
    _grid(tmp_path, "473067", "DEM10B", "fguuid:jgd2000.bl", 200.0)
    _grid(tmp_path, "473067", "DEM10A", "fguuid:jgd2000.bl", 250.0, half)  # volcano map over part of 473067
    _grid(tmp_path, "473065", "DEM10B", "fguuid:jgd2011.bl", 300.0)
    stack = stacks.load().stack("dem10")
    tree = sources.SourceTree(str(tmp_path / "src"))
    ms = build.build_primary(stack, "4730", tree, str(tmp_path / "out"), log=lambda *a: None)
    assert [(m["key"], m["label"], len(m["grids"])) for m in ms] == [
        ("base/dem10/4730.tif", "jgd2000", 3), ("base/dem10/4730-fill.tif", "jgd2011", 1)]
    main = tmp_path / "out" / "base" / "dem10" / "4730.tif"
    with rasterio.open(main) as d:
        assert d.crs.to_epsg() == 4612 and d.nodata == -9999 and d.block_shapes[0] == (512, 512)
        assert d.transform.a == pytest.approx(1 / 9000)
        r0 = round((d.transform.f - meshcode.bounds("473067")[3]) / -d.transform.e)
        c0 = round((meshcode.bounds("473067")[0] - d.transform.c) / d.transform.a)
        a = d.read(1, window=((r0, r0 + 750), (c0, c0 + 1125)))
    assert (a[:, :500] == 250).all() and (a[:, 500:] == 200).all()  # DEM10A on top where present
    m = ms[0]
    assert m["paint_order"] == ["DEM10B", "DEM10A"] and [g["product"] for g in m["grids"]] == ["DEM10B", "DEM10B", "DEM10A"]
    assert m["secondary_meshes"]["473067"] == {
        "DEM10A": {"labels": {"jgd2000": 1}, "editions": ["2016-10-01"], "grids": 1},
        "DEM10B": {"labels": {"jgd2000": 1}, "editions": ["2016-10-01"], "grids": 1}}
    assert m["sha256"] == build.file_hashes(str(main))[0]
    rows = build.labels_from_manifests([str(main) + ".manifest.json", str(main).replace(".tif", "-fill.tif") + ".manifest.json"])
    assert {(r["mesh6"], r["product"], tuple(r["labels"]), r["key"]) for r in rows} >= {("473065", "DEM10B", ("jgd2011",), "base/dem10/4730-fill.tif")}
    with pytest.raises(build.BuildError, match="exists"):
        build.build_primary(stack, "4730", tree, str(tmp_path / "out"), log=lambda *a: None)


@pytest.mark.skipif(not GDAL, reason="needs the GDAL CLI")
def test_build_refuses_wrong_pixel(tmp_path):
    import rasterio
    from rasterio.transform import from_origin

    d = tmp_path / "src" / "dem10b"
    d.mkdir(parents=True)
    with rasterio.open(d / "FG-GML-4730-67-DEM10B-20161001.tif", "w", driver="GTiff", height=10, width=10, count=1, dtype="float32",
                       crs="EPSG:4612", transform=from_origin(130.875, 31.9166, 0.001, 0.001), nodata=-9999) as ds:
        ds.write(np.ones((1, 10, 10), np.float32))
    with pytest.raises(build.BuildError, match="pixel"):
        build.build_primary(stacks.load().stack("dem10"), "4730", sources.SourceTree(str(tmp_path / "src")), str(tmp_path / "out"))


def test_provenance_round_trip(tmp_path):
    (tmp_path / "provenance.jsonl").write_text(json.dumps({"file": "dem10b/FG-GML-4730-67-DEM10B-20161001.tif", "label": "jgd2011"}) + "\n")
    assert build.load_provenance(str(tmp_path))["FG-GML-4730-67-DEM10B-20161001.tif"]["label"] == "jgd2011"


def test_secondary_summary_groups_tertiaries():
    rows = [{"mesh": "47302537", "product": "DEM5A", "label": "jgd2011", "edition": "2024-12-13"},
            {"mesh": "47302538", "product": "DEM5A", "label": "jgd2011", "edition": "2016-10-01"}]
    assert build.secondary_summary(rows) == {"473025": {"DEM5A": {"labels": {"jgd2011": 2}, "editions": ["2016-10-01", "2024-12-13"], "grids": 2}}}


def test_plan_grid_label():
    assert plan.Grid("n", "473067", "DEM10B", "2016-10-01", 4612).label == "jgd2000"



@pytest.mark.skipif(not GDAL, reason="needs the GDAL CLI")
@pytest.mark.parametrize("dx, size, match", [(1 / 9000, (1125, 750), "origin"), (0.0, (1125, 700), "expected 1125x750")])
def test_build_refuses_grid_off_its_mesh(tmp_path, dx, size, match):
    import rasterio
    from rasterio.transform import from_origin

    w, _, _, n = meshcode.bounds("473067")
    d = tmp_path / "src" / "dem10b"
    d.mkdir(parents=True)
    with rasterio.open(d / "FG-GML-4730-67-DEM10B-20161001.tif", "w", driver="GTiff", height=size[1], width=size[0], count=1,
                       dtype="float32", crs="EPSG:4612", transform=from_origin(w + dx, n, 1 / 9000, 1 / 9000), nodata=-9999) as ds:
        ds.write(np.ones((1, size[1], size[0]), np.float32))
    with pytest.raises(build.BuildError, match=match):
        build.build_primary(stacks.load().stack("dem10"), "4730", sources.SourceTree(str(tmp_path / "src")), str(tmp_path / "out"))
