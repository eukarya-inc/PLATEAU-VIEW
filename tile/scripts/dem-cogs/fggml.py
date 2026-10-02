"""基盤地図情報 数値標高モデル (FG-GML / JPGIS 2.1 GML) -> per-grid GeoTIFF.

GDAL has no driver for these XMLs. One XML holds one grid (a secondary mesh
for DEM10A/B, a tertiary mesh for DEM5A/B/C and DEM1A):

* ``gml:Envelope`` ``srsName`` names the datum (``fguuid:jgd2000.bl`` /
  ``jgd2011.bl`` / ``jgd2024.bl``); ``lowerCorner``/``upperCorner`` are
  ``lat lon`` of the grid's outer edges (pixel corners, not centres).
* ``gml:GridEnvelope`` ``low``/``high`` give the size, ``gml:sequenceRule``
  must be ``+x-y`` (row-major from the north-west), ``gml:startPoint`` (if
  present) is the ``x y`` of the first tuple -- leading cells are omitted.
* ``gml:tupleList`` lines are ``<種別>,<height>``; trailing cells may be
  omitted. Missing cells and ``-9999.`` are no-data.

The GeoTIFF is float32, nodata -9999, the Envelope as its extent, EPSG 4612
or 6668 from ``srsName``. For DEM10B this reproduces the FME-converted grids
in the backup pixel for pixel (same CRS, geotransform, mask and values; checked
on 21 secondary meshes). ``jgd2024.bl`` grids are refused: the JGD2011-era
stack must not contain them and PROJ has no geographic 2D EPSG code for them.
"""

from __future__ import annotations

import collections
import hashlib
import io
import os
import xml.etree.ElementTree as ET
import zipfile
from dataclasses import dataclass, field

import numpy as np

import meshcode

NS = {"gml": "http://www.opengis.net/gml/3.2", "f": "http://fgd.gsi.go.jp/spec/2008/FGD_GMLSchema"}
NODATA = -9999.0
# Grid size per product (columns, rows).
SHAPES = {"DEM10A": (1125, 750), "DEM10B": (1125, 750), "DEM5A": (225, 150), "DEM5B": (225, 150), "DEM5C": (225, 150), "DEM1A": (1125, 750)}


class GmlError(ValueError):
    pass


@dataclass
class Grid:
    mesh: str
    product: str
    dev_date: str | None
    srs_name: str
    bounds: tuple[float, float, float, float]  # west, south, east, north
    data: np.ndarray  # float32, rows north -> south
    kinds: dict = field(default_factory=dict)  # tuple 種別 -> count

    @property
    def label(self) -> str:
        return meshcode.label_of_srs(self.srs_name)

    @property
    def valid_px(self) -> int:
        return int((self.data > NODATA + 1).sum())


def _text(root, path: str) -> str | None:
    e = root.find(path, NS)
    return e.text.strip() if e is not None and e.text else None


def parse(xml_bytes: bytes, product: str = "") -> Grid:
    """Parse one DEM XML. ``product`` (from the file name; the XML does not
    name it) enables the grid-size check."""
    root = ET.fromstring(xml_bytes)
    dem = root.find("f:DEM", NS)
    if dem is None:
        raise GmlError("no f:DEM element")
    mesh = _text(dem, "f:mesh")
    dev = _text(dem, "f:devDate/gml:timePosition")
    env = root.find(".//gml:Envelope", NS)
    if env is None or mesh is None:
        raise GmlError("no gml:Envelope or f:mesh")
    srs = env.get("srsName")
    lo = [float(v) for v in _text(env, "gml:lowerCorner").split()]
    hi = [float(v) for v in _text(env, "gml:upperCorner").split()]
    ge = root.find(".//gml:GridEnvelope", NS)
    low = [int(v) for v in _text(ge, "gml:low").split()]
    high = [int(v) for v in _text(ge, "gml:high").split()]
    ncols, nrows = high[0] - low[0] + 1, high[1] - low[1] + 1
    rule = root.find(".//gml:sequenceRule", NS)
    if rule is None or rule.get("order") != "+x-y":
        raise GmlError(f"{mesh}: sequenceRule order {rule.get('order') if rule is not None else None!r}, expected '+x-y'")
    sp = _text(root, ".//gml:startPoint")
    sx, sy = (int(v) for v in sp.split()) if sp else (0, 0)
    kinds: collections.Counter = collections.Counter()
    vals = []
    for line in (_text(root, ".//gml:tupleList") or "").splitlines():
        if "," not in line:
            continue
        k, v = line.split(",", 1)
        kinds[k.strip()] += 1
        vals.append(float(v))
    arr = np.asarray(vals, dtype=np.float32)
    grid = np.full(nrows * ncols, NODATA, dtype=np.float32)
    start = sy * ncols + sx
    if start + arr.size > grid.size:
        raise GmlError(f"{mesh}: {arr.size} tuples from start {start} overflow a {ncols}x{nrows} grid")
    grid[start : start + arr.size] = arr
    grid = grid.reshape(nrows, ncols)
    grid[grid <= NODATA + 1] = NODATA
    g = Grid(mesh, (product or "").upper(), dev, srs, (lo[1], lo[0], hi[1], hi[0]), grid, dict(kinds))
    _check(g)
    return g


def _check(g: Grid) -> None:
    meshcode.label_of_srs(g.srs_name)  # unknown srsName -> error
    want = meshcode.bounds(g.mesh)
    if any(abs(a - b) > 1e-6 for a, b in zip(g.bounds, want, strict=True)):
        raise GmlError(f"{g.mesh}: Envelope {g.bounds} is not the mesh {want}")
    if g.product and g.product in SHAPES and (g.data.shape[1], g.data.shape[0]) != SHAPES[g.product]:
        raise GmlError(f"{g.mesh}: {g.product} grid is {g.data.shape[1]}x{g.data.shape[0]}, expected {SHAPES[g.product]}")


def write_geotiff(g: Grid, path: str) -> None:
    import rasterio
    from rasterio.transform import from_bounds

    if g.label not in meshcode.LABEL_EPSG:
        raise GmlError(f"{g.mesh}: {g.srs_name} grids are not accepted into this stack (no EPSG code / not JGD2011-era)")
    h, w = g.data.shape
    tmp = path + ".part"
    with rasterio.open(
        tmp, "w", driver="GTiff", height=h, width=w, count=1, dtype="float32",
        crs=f"EPSG:{meshcode.LABEL_EPSG[g.label]}", transform=from_bounds(*g.bounds, w, h),
        nodata=NODATA, compress="deflate",
    ) as d:
        d.write(g.data, 1)
    os.replace(tmp, path)


def sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def zip_to_tifs(zip_path: str, src_root: str) -> list[dict]:
    """Every XML of one GSI zip -> ``src_root/<product>/<xml name>.tif``; provenance rows.

    The product comes from the XML file name (``FG-GML-4730-67-DEM10B-20161001.xml``).
    A zip without any XML is an error.
    """
    with open(zip_path, "rb") as f:
        blob = f.read()
    zsha = sha256_bytes(blob)
    rows = []
    with zipfile.ZipFile(io.BytesIO(blob)) as z:
        names = z.namelist()
        xmls = [n for n in names if n.lower().endswith(".xml")]
        if not xmls:
            raise GmlError(f"{zip_path}: no XML inside ({names[:5]})")
        for n in sorted(xmls):
            data = z.read(n)
            base = os.path.basename(n)
            gname = meshcode.parse_name(base[:-4] + ".tif")
            g = parse(data, gname.product)
            if g.mesh != gname.mesh:
                raise GmlError(f"{zip_path}:{n}: XML mesh {g.mesh} != file name mesh {gname.mesh}")
            out_dir = os.path.join(src_root, gname.product.lower())
            os.makedirs(out_dir, exist_ok=True)
            out = os.path.join(out_dir, gname.name)
            write_geotiff(g, out)
            rows.append({
                "file": f"{gname.product.lower()}/{gname.name}",
                "mesh": g.mesh, "product": gname.product, "edition": gname.edition,
                "dev_date": g.dev_date, "srs_name": g.srs_name, "label": g.label,
                "valid_px": g.valid_px, "kinds": g.kinds,
                "zip": os.path.basename(zip_path), "zip_sha256": zsha, "zip_bytes": len(blob),
                "xml": n, "xml_sha256": sha256_bytes(data),
            })
    return rows
