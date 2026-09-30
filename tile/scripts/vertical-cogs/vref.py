"""Shared helpers for the vertical-reference COG tooling.

Everything here is deliberately plain numpy + stdlib so that the node-lattice
arithmetic can be read (and re-implemented) without chasing library behaviour.

Conventions used throughout:

* A *node lattice* is a regular lat/lon grid of values that live exactly on
  grid nodes (GSI's PatchJGD(H) parameters, GSI geoid grids).
* Rasters are written so that **pixel centres coincide with nodes**: the
  geotransform origin is the north-west node shifted half a cell further
  north-west. A reader that uses the ordinary pixel-is-area convention and
  interpolates bilinearly between pixel centres therefore reproduces
  node-bilinear interpolation exactly (see sampler.py for the normative rule).
* Arrays are stored north-up (row 0 = northernmost node row), as GeoTIFF wants.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import tempfile
from dataclasses import dataclass

import numpy as np

# ---------------------------------------------------------------------------
# JIS X 0410 mesh codes
# ---------------------------------------------------------------------------
# Primary mesh:   40' lat x 1 deg lon       code pp qq
# Secondary mesh: 5'  lat x 7.5' lon        code pp qq r s   (r, s in 0..7)
# Tertiary mesh:  30" lat x 45" lon         code pp qq r s t u (t, u in 0..9)
#
# Integer node indices: i counts 30" steps of latitude from 0 deg, j counts
# 45" steps of longitude from 100 deg E. A tertiary mesh's SW corner is node
# (i, j) with i = pp*80 + r*10 + t and j = qq*80 + s*10 + u.

LAT_STEP_DEG = 30.0 / 3600.0  # 1/120 deg
LON_STEP_DEG = 45.0 / 3600.0  # 1/80 deg
LON_ORIGIN_DEG = 100.0


def mesh8_to_node(code: int) -> tuple[int, int]:
    """8-digit tertiary mesh code -> (i, j) index of its SW corner node."""
    pp = code // 1000000
    qq = code // 10000 % 100
    r = code // 1000 % 10
    s = code // 100 % 10
    t = code // 10 % 10
    u = code % 10
    if r > 7 or s > 7:
        raise ValueError(f"not a valid tertiary mesh code: {code}")
    return pp * 80 + r * 10 + t, qq * 80 + s * 10 + u


def node_to_mesh8(i: int, j: int) -> int:
    pp, rem = divmod(i, 80)
    r, t = divmod(rem, 10)
    qq, rem = divmod(j, 80)
    s, u = divmod(rem, 10)
    return ((((pp * 100 + qq) * 10 + r) * 10 + s) * 10 + t) * 10 + u


def node_to_mesh6(i: int, j: int) -> int:
    """Secondary mesh that contains the cell whose SW corner is node (i, j)."""
    return node_to_mesh8(i, j) // 100


def mesh6_sw_node(code: int) -> tuple[int, int]:
    return mesh8_to_node(code * 100)


# ---------------------------------------------------------------------------
# PatchJGD(H) parameter files
# ---------------------------------------------------------------------------


@dataclass
class ParFile:
    header: list[str]  # decoded header lines (CP932)
    version: str  # e.g. "Ver.1.0.0"
    nodes: dict[tuple[int, int], float]  # (i, j) -> dH [m]


def read_par(path: str) -> ParFile:
    """Parse a PatchJGD(H) .par file.

    Layout (as published by GSI): CP932 text; line 1 is the format banner
    ("for PatchJGD(H)  Ver.x.y.z  ..."), lines 2-15 are free text, line 16 is
    the column header "MeshCode dH(m) 0.00000", and every following line is
    "<8-digit tertiary mesh> <dH> 0.00000". We do not trust a fixed header
    length: a data row is any row whose first token is an 8-digit integer.
    """
    header: list[str] = []
    nodes: dict[tuple[int, int], float] = {}
    with open(path, "rb") as f:
        for raw in f:
            line = raw.decode("cp932").rstrip("\r\n")
            tok = line.split()
            if len(tok) >= 2 and len(tok[0]) == 8 and tok[0].isdigit():
                key = mesh8_to_node(int(tok[0]))
                if key in nodes:
                    raise ValueError(f"{path}: duplicate mesh {tok[0]}")
                nodes[key] = float(tok[1])
            elif not nodes:
                header.append(line)
            elif line.strip():
                raise ValueError(f"{path}: unexpected line after data: {line!r}")
    banner = header[0].split() if header else []
    version = next((t for t in banner if t.startswith("Ver.")), "")
    return ParFile(header=header, version=version, nodes=nodes)


# ---------------------------------------------------------------------------
# Node lattice -> north-up array
# ---------------------------------------------------------------------------


@dataclass
class NodeGrid:
    """A node lattice held as a north-up float32 array (NaN = no value).

    ``lat_n`` / ``lon_w`` are the coordinates of the node stored at [0, 0]
    (north-west corner node); ``dlat`` / ``dlon`` the node spacing in degrees.
    ``lat_n`` etc. are kept as exact rationals where possible via ``*_num`` /
    ``*_den`` so metadata can state them without rounding noise.
    """

    data: np.ndarray  # shape (rows, cols), float32, row 0 = north
    lat_n: float
    lon_w: float
    dlat: float
    dlon: float

    @property
    def rows(self) -> int:
        return self.data.shape[0]

    @property
    def cols(self) -> int:
        return self.data.shape[1]

    def geotransform(self) -> tuple[float, float, float, float, float, float]:
        """GDAL geotransform with pixel centres on the nodes (pixel-is-area)."""
        return (
            self.lon_w - self.dlon / 2.0,
            self.dlon,
            0.0,
            self.lat_n + self.dlat / 2.0,
            0.0,
            -self.dlat,
        )


def mesh_nodes_to_grid(nodes: dict[tuple[int, int], float]) -> NodeGrid:
    """Place a mesh-indexed node dict on the tight bounding lattice."""
    ii = np.fromiter((k[0] for k in nodes), dtype=np.int64)
    jj = np.fromiter((k[1] for k in nodes), dtype=np.int64)
    vv = np.fromiter(nodes.values(), dtype=np.float64)
    i_min, i_max, j_min, j_max = ii.min(), ii.max(), jj.min(), jj.max()
    return mesh_nodes_to_grid_bbox(nodes, (int(i_min), int(i_max), int(j_min), int(j_max)), ii, jj, vv)


def mesh_nodes_to_grid_bbox(nodes, bbox, ii=None, jj=None, vv=None) -> NodeGrid:
    i_min, i_max, j_min, j_max = bbox
    if ii is None:
        ii = np.fromiter((k[0] for k in nodes), dtype=np.int64)
        jj = np.fromiter((k[1] for k in nodes), dtype=np.int64)
        vv = np.fromiter(nodes.values(), dtype=np.float64)
    rows = i_max - i_min + 1
    cols = j_max - j_min + 1
    a = np.full((rows, cols), np.nan, dtype=np.float32)
    a[i_max - ii, jj - j_min] = vv.astype(np.float32)
    return NodeGrid(
        data=a,
        lat_n=i_max * LAT_STEP_DEG,
        lon_w=LON_ORIGIN_DEG + j_min * LON_STEP_DEG,
        dlat=LAT_STEP_DEG,
        dlon=LON_STEP_DEG,
    )


# ---------------------------------------------------------------------------
# Geoid grids
# ---------------------------------------------------------------------------


def parse_dms(s: str) -> float:
    """'15°00\\'00"' -> 15.0 (ISG 2.0 header angles)."""
    s = s.strip()
    d, rest = s.split("°", 1)
    m, rest = rest.split("'", 1)
    sec = rest.rstrip('"').strip() or "0"
    sign = -1.0 if d.strip().startswith("-") else 1.0
    return sign * (abs(float(d)) + float(m) / 60.0 + float(sec) / 3600.0)


@dataclass
class GeoidSource:
    grid: NodeGrid
    header: dict[str, str]
    model: str


def read_isg(path: str) -> GeoidSource:
    """ISG 2.0 (JPGEO2024 family). Rows run N->S, columns W->E.

    ISG 2.0 with ``coord type = geodetic`` and ``data ordering = N-to-S,
    W-to-E`` stores values *on grid nodes*: the first value is the node at
    (lat max, lon min). We verify the header says so rather than assume it.
    """
    header: dict[str, str] = {}
    with open(path, encoding="utf-8", errors="replace") as f:
        lines = iter(f)
        for line in lines:
            if line.strip().startswith("begin_of_head"):
                break
        for line in lines:
            t = line.strip()
            if t.startswith("end_of_head"):
                break
            for sep in ("=", ":"):
                if sep in t:
                    k, v = t.split(sep, 1)
                    header[k.strip()] = v.strip()
                    break
        vals = np.loadtxt(lines, dtype=np.float64, ndmin=2)
    nrows = int(header["nrows"])
    ncols = int(header["ncols"])
    if vals.shape != (nrows, ncols):
        raise ValueError(f"{path}: data {vals.shape} != header {(nrows, ncols)}")
    if "N-to-S" not in header.get("data ordering", ""):
        raise ValueError(f"{path}: unexpected data ordering {header.get('data ordering')}")
    if header.get("coord type", "geodetic") != "geodetic":
        raise ValueError(f"{path}: coord type {header.get('coord type')}")
    lat_min = parse_dms(header["lat min"])
    lat_max = parse_dms(header["lat max"])
    lon_min = parse_dms(header["lon min"])
    lon_max = parse_dms(header["lon max"])
    dlat = parse_dms(header["delta lat"])
    dlon = parse_dms(header["delta lon"])
    # Node-registered grid: extent spans (n-1) intervals.
    if abs((lat_max - lat_min) / dlat - (nrows - 1)) > 1e-6 or abs((lon_max - lon_min) / dlon - (ncols - 1)) > 1e-6:
        raise ValueError(f"{path}: header extent is not node-registered")
    nodata = float(header.get("nodata", "-9999"))
    a = vals.astype(np.float32)
    a[vals == nodata] = np.nan
    return GeoidSource(
        grid=NodeGrid(data=a, lat_n=lat_max, lon_w=lon_min, dlat=dlat, dlon=dlon),
        header=header,
        model=header.get("model name", ""),
    )


def read_gsigeo_asc(path: str) -> GeoidSource:
    """GSIGEO2011 ASC. Header: lat0 lon0 dlat dlon nlat nlon ikind version.

    Rows run S->N (first row = lat0), columns W->E; each latitude row is
    wrapped over several text lines. 999.0000 = no data. Values are on nodes
    (GSI's reference Fortran interpolates bilinearly between them).
    """
    with open(path, encoding="ascii") as f:
        head = f.readline().split()
        vals = np.array(f.read().split(), dtype=np.float64)
    lat0, lon0 = float(head[0]), float(head[1])
    nlat, nlon = int(head[4]), int(head[5])
    # 0.016667 / 0.025000 are rounded prints of 1' and 1.5'.
    if head[2] != "0.016667" or head[3] != "0.025000":
        raise ValueError(f"{path}: unexpected spacing {head[2:4]}")
    if vals.size != nlat * nlon:
        raise ValueError(f"{path}: {vals.size} values != {nlat}x{nlon}")
    a = vals.reshape(nlat, nlon)[::-1].astype(np.float32)  # -> north-up
    a[vals.reshape(nlat, nlon)[::-1] == 999.0] = np.nan
    dlat, dlon = 1.0 / 60.0, 1.5 / 60.0
    return GeoidSource(
        grid=NodeGrid(data=a, lat_n=lat0 + (nlat - 1) * dlat, lon_w=lon0, dlat=dlat, dlon=dlon),
        header={"lat0": head[0], "lon0": head[1], "dlat": head[2], "dlon": head[3], "nlat": head[4], "nlon": head[5], "ikind": head[6], "version": head[7] if len(head) > 7 else ""},
        model="GSIGEO2011 " + (head[7] if len(head) > 7 else ""),
    )


# ---------------------------------------------------------------------------
# COG writer (GDAL CLI)
# ---------------------------------------------------------------------------

COG_OPTS = [
    "-co", "COMPRESS=ZSTD",
    "-co", "PREDICTOR=3",
    "-co", "RESAMPLING=NEAREST",
    "-co", "BLOCKSIZE=512",
    "-co", "NUM_THREADS=ALL_CPUS",
]


def write_cog(grid: NodeGrid, out: str, metadata: dict[str, str], description: str) -> None:
    """Write ``grid`` as a float32 COG with NaN nodata via a raw VRT.

    Going through a VRT keeps the geotransform we computed bit-for-bit (no
    driver re-derives it from bounds) and needs only the GDAL CLI.
    """
    gt = grid.geotransform()
    with tempfile.TemporaryDirectory() as td:
        raw = os.path.join(td, "grid.f32")
        grid.data.astype("<f4").tofile(raw)
        vrt = os.path.join(td, "grid.vrt")
        with open(vrt, "w") as f:
            f.write(
                f'<VRTDataset rasterXSize="{grid.cols}" rasterYSize="{grid.rows}">\n'
                f"  <SRS>EPSG:6668</SRS>\n"
                f"  <GeoTransform>{gt[0]!r}, {gt[1]!r}, {gt[2]!r}, {gt[3]!r}, {gt[4]!r}, {gt[5]!r}</GeoTransform>\n"
                f'  <VRTRasterBand dataType="Float32" band="1" subClass="VRTRawRasterBand">\n'
                f"    <NoDataValue>nan</NoDataValue>\n"
                f"    <SourceFilename relativeToVRT=\"1\">grid.f32</SourceFilename>\n"
                f"    <ByteOrder>LSB</ByteOrder>\n"
                f"    <ImageOffset>0</ImageOffset><PixelOffset>4</PixelOffset><LineOffset>{4 * grid.cols}</LineOffset>\n"
                f"  </VRTRasterBand>\n"
                f"</VRTDataset>\n"
            )
        cmd = ["gdal_translate", "-q", "-of", "COG", *COG_OPTS]
        for k, v in metadata.items():
            cmd += ["-mo", f"{k}={v}"]
        cmd += ["-mo", f"TIFFTAG_IMAGEDESCRIPTION={description}", vrt, out]
        subprocess.run(cmd, check=True)


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def write_json(path: str, obj) -> None:
    with open(path, "w", encoding="utf-8") as f:
        json.dump(obj, f, ensure_ascii=False, indent=2)
        f.write("\n")
