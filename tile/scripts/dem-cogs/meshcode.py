"""GSI mesh codes, 基盤地図情報 DEM file names and datum labels (pure functions).

Mesh codes (JIS X 0410):

* primary   ``PPQQ``      40' (lat) x 1 deg (lon), south-west corner (PP/1.5, QQ+100)
* secondary ``PPQQrc``    5' x 7.5' (r = row 0-7 northwards, c = column 0-7 eastwards)
* tertiary  ``PPQQrcRC``  30" x 45" (R, C = 0-9)

DEM10A/B ship one grid per secondary mesh (1125 x 750 px, 1/9000 deg), DEM5A/B/C
and DEM1A one grid per tertiary mesh (225 x 150 px at 1/18000 deg; 1125 x 750
at 1/90000 deg). The per-grid GeoTIFFs this pipeline reads and writes are named
like the FME output in the backup bucket, which is the XML name with ``.tif``::

    FG-GML-4730-67-DEM10B-20161001.tif      (secondary: PPQQ-rc)
    FG-GML-4730-25-37-DEM5A-20241213.tif    (tertiary:  PPQQ-rc-RC)

The product token's case varies in the backup (``dem10b``, ``DEM10B``,
``dem10B``); it is normalised to upper case.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

# srsName in the FG-GML <gml:Envelope> -> label used throughout this pipeline.
SRS_LABEL = {
    "fguuid:jgd2000.bl": "jgd2000",
    "fguuid:jgd2011.bl": "jgd2011",
    "fguuid:jgd2024.bl": "jgd2024",
}
# Geographic 2D EPSG code written into the GeoTIFFs (and back). JGD2024 has no
# geographic 2D EPSG code in the PROJ database this was written against, and
# the served (JGD2011-era) stack must never contain it: gml2tif refuses it.
LABEL_EPSG = {"jgd2000": 4612, "jgd2011": 6668}
EPSG_LABEL = {v: k for k, v in LABEL_EPSG.items()}
# When two CRS groups of a primary mesh have the same number of grids, the
# newer datum becomes the main COG (see plan.split_by_crs).
LABEL_RANK = {"jgd2000": 0, "jgd2011": 1}

NAME_RE = re.compile(r"^FG-GML-(\d{4})-(\d{2})(?:-(\d{2}))?-(DEM[0-9]+[A-Z])-(\d{8})\.tif$", re.IGNORECASE)


class MeshError(ValueError):
    pass


@dataclass(frozen=True)
class GridName:
    """A parsed per-grid file name."""

    name: str
    mesh: str  # 6 (secondary) or 8 (tertiary) digits
    product: str  # DEM10A, DEM10B, DEM5A, DEM5B, DEM5C, DEM1A
    edition: str  # YYYY-MM-DD (作成年月日, the last 8 digits of the XML name)

    @property
    def primary(self) -> str:
        return self.mesh[:4]

    @property
    def secondary(self) -> str:
        return self.mesh[:6]


def parse_name(name: str) -> GridName:
    m = NAME_RE.match(name)
    if not m:
        raise MeshError(f"not a 基盤地図情報 DEM grid file name: {name!r}")
    mesh = m[1] + m[2] + (m[3] or "")
    check_mesh(mesh)
    d = m[5]
    return GridName(name, mesh, m[4].upper(), f"{d[:4]}-{d[4:6]}-{d[6:]}")


def grid_name(mesh: str, product: str, edition: str) -> str:
    """Inverse of :func:`parse_name` (upper-case product, ``.tif``)."""
    check_mesh(mesh)
    parts = [mesh[:4], mesh[4:6]] + ([mesh[6:8]] if len(mesh) == 8 else [])
    return f"FG-GML-{'-'.join(parts)}-{product.upper()}-{edition.replace('-', '')}.tif"


def check_mesh(mesh: str) -> None:
    if not re.fullmatch(r"\d{4}(\d{2}(\d{2})?)?", mesh):
        raise MeshError(f"bad mesh code {mesh!r}")
    if len(mesh) >= 6 and (int(mesh[4]) > 7 or int(mesh[5]) > 7):
        raise MeshError(f"bad secondary mesh code {mesh!r} (row/column must be 0-7)")


def bounds(mesh: str) -> tuple[float, float, float, float]:
    """(west, south, east, north) in degrees of a primary/secondary/tertiary mesh."""
    check_mesh(mesh)
    south = int(mesh[:2]) / 1.5
    west = int(mesh[2:4]) + 100.0
    dlat, dlon = 2 / 3, 1.0
    if len(mesh) >= 6:
        dlat, dlon = dlat / 8, dlon / 8
        south += int(mesh[4]) * dlat
        west += int(mesh[5]) * dlon
    if len(mesh) == 8:
        dlat, dlon = dlat / 10, dlon / 10
        south += int(mesh[6]) * dlat
        west += int(mesh[7]) * dlon
    return west, south, west + dlon, south + dlat


def label_of_srs(srs_name: str) -> str:
    try:
        return SRS_LABEL[srs_name]
    except KeyError:
        raise MeshError(f"unknown srsName {srs_name!r}") from None


def label_of_epsg(epsg: int | None) -> str:
    if epsg not in EPSG_LABEL:
        raise MeshError(f"EPSG:{epsg} is not a datum this pipeline knows ({sorted(EPSG_LABEL)})")
    return EPSG_LABEL[epsg]
