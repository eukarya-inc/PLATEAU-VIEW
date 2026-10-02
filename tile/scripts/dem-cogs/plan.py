"""Pure planning logic: which grids go into which COG, in which order.

Rules (each one exists because the original batch got it wrong silently):

1. **CRS split.** ``gdalbuildvrt`` keeps the CRS of the first input and drops
   every input with another CRS with only a warning and exit code 0. DEM10B
   secondary meshes are labelled ``jgd2000`` or ``jgd2011`` individually, so a
   primary mesh can hold both. The grids of a primary are split by EPSG: the
   larger group becomes ``<mesh>.tif`` (*main*), the other ``<mesh>-fill.tif``
   (*fill*); on a tie the newer datum is main. A third CRS (or an unknown one)
   is refused. ``main_epsg`` pins the main group explicitly (the served stack
   has four primaries whose main COG is the smaller group, see README).
2. **Explicit paint order.** Inside one COG the VRT lists the grids bottom ->
   top in the stack's product order (``DEM10B`` < ``DEM10A``,
   ``DEM5C`` < ``DEM5B`` < ``DEM5A``), then by mesh code, so the better product
   wins where two cover the same pixel. The original batch passed ``find``
   output, whose order is arbitrary.
3. **Layering across COGs.** The Worker paints ``<mesh>-fill.tif`` under
   ``<mesh>.tif`` (keys sort ``-`` before ``.``). If a mesh's higher product
   lands in the fill and a lower product of the same mesh in the main COG,
   the lower one would win: refused (:func:`check_layering`).
4. **One grid per (product, mesh).** Two editions of the same grid are refused.
5. **Every input reaches the VRT.** :func:`check_vrt_sources` compares the
   number of ``<SourceFilename>`` entries with the number of inputs.
"""

from __future__ import annotations

import collections
import re
from dataclasses import dataclass

import meshcode


class PlanError(ValueError):
    pass


@dataclass(frozen=True)
class Grid:
    """A source grid as far as planning is concerned."""

    name: str
    mesh: str
    product: str
    edition: str
    epsg: int | None
    path: str = ""

    @property
    def label(self) -> str:
        return meshcode.label_of_epsg(self.epsg)


@dataclass(frozen=True)
class Group:
    role: str  # "main" | "fill"
    epsg: int
    grids: tuple[Grid, ...]  # VRT order, bottom -> top

    @property
    def label(self) -> str:
        return meshcode.label_of_epsg(self.epsg)


def order(grids: list[Grid], products: tuple[str, ...]) -> list[Grid]:
    """Bottom -> top: product order of the stack, then mesh code, then name."""
    rank = {p: i for i, p in enumerate(products)}
    for g in grids:
        if g.product not in rank:
            raise PlanError(f"{g.name}: product {g.product} is not in this stack {products}")
    return sorted(grids, key=lambda g: (rank[g.product], g.mesh, g.name))


def check_unique(grids: list[Grid]) -> None:
    seen: dict[tuple[str, str], str] = {}
    for g in grids:
        k = (g.product, g.mesh)
        if k in seen:
            raise PlanError(f"two grids for {g.product} {g.mesh}: {seen[k]} and {g.name}; keep exactly one edition")
        seen[k] = g.name


def split_by_crs(grids: list[Grid], main_epsg: int | None = None) -> list[tuple[str, int, list[Grid]]]:
    """[(role, epsg, grids)], main first. Refuses unknown CRS and more than two."""
    if not grids:
        raise PlanError("no input grids")
    by: dict[int, list[Grid]] = collections.defaultdict(list)
    for g in grids:
        if g.epsg not in meshcode.EPSG_LABEL:
            raise PlanError(f"{g.name}: CRS EPSG:{g.epsg} is not one of {sorted(meshcode.EPSG_LABEL)} -- refusing to guess")
        by[g.epsg].append(g)
    if len(by) > 2:
        raise PlanError(f"{len(by)} CRSs in one primary mesh ({ {meshcode.EPSG_LABEL[e]: len(v) for e, v in by.items()} }) -- refusing")
    if main_epsg is not None:
        if main_epsg not in by:
            raise PlanError(f"pinned main CRS EPSG:{main_epsg} has no grids here (have {sorted(by)})")
        main = main_epsg
    else:
        main = max(by, key=lambda e: (len(by[e]), meshcode.LABEL_RANK[meshcode.EPSG_LABEL[e]]))
    out = [("main", main, by[main])]
    out += [("fill", e, v) for e, v in by.items() if e != main]
    return out


def check_layering(groups: list[Group], products: tuple[str, ...]) -> None:
    """A mesh's best product must not sit in a COG painted under a worse one."""
    rank = {p: i for i, p in enumerate(products)}
    level = {"fill": 0, "main": 1}  # paint order of the two COGs
    where: dict[str, list[tuple[int, int, str]]] = collections.defaultdict(list)
    for gr in groups:
        for g in gr.grids:
            where[g.mesh].append((rank[g.product], level[gr.role], g.name))
    bad = []
    for mesh, xs in where.items():
        xs.sort()
        # product rank must be non-decreasing in paint level: no higher product below a lower one
        for lo in xs:
            for hi in xs:
                if hi[0] > lo[0] and hi[1] < lo[1]:
                    bad.append(f"{mesh}: {hi[2]} (fill) would be painted under {lo[2]} (main)")
    if bad:
        raise PlanError("cross-COG paint order violates the product order: " + "; ".join(bad))


def plan_primary(grids: list[Grid], products: tuple[str, ...], main_epsg: int | None = None) -> list[Group]:
    check_unique(grids)
    groups = [Group(role, epsg, tuple(order(gs, products))) for role, epsg, gs in split_by_crs(grids, main_epsg)]
    check_layering(groups, products)
    return groups


def check_vrt_sources(vrt_xml: str, inputs: list[str]) -> None:
    """Every input must appear in the VRT exactly once and in the given order."""
    got = re.findall(r"<SourceFilename[^>]*>([^<]+)</SourceFilename>", vrt_xml)
    if len(got) != len(inputs):
        raise PlanError(f"VRT has {len(got)} sources for {len(inputs)} inputs -- gdalbuildvrt dropped inputs, refusing")
    for i, (a, b) in enumerate(zip(got, inputs, strict=True)):
        if not (a == b or a.endswith("/" + b.rsplit("/", 1)[-1])):
            raise PlanError(f"VRT source #{i} is {a!r}, expected {b!r} -- order not preserved, refusing")


def buildvrt_warnings(stderr: str) -> list[str]:
    """gdalbuildvrt messages that mean an input was not used as given."""
    return [l for l in stderr.splitlines() if re.search(r"warning|skip|error", l, re.IGNORECASE)]


def parse_main_pins(spec: str | None) -> dict[str, int]:
    """``"3623=6668,6545=6668"`` -> {"3623": 6668, ...}."""
    out: dict[str, int] = {}
    for part in (spec or "").split(","):
        part = part.strip()
        if not part:
            continue
        m = re.fullmatch(r"(\d{4})=(?:EPSG:)?(\d+)", part)
        if not m:
            raise PlanError(f"bad --main-crs entry {part!r} (want <mesh4>=<epsg>)")
        out[m[1]] = int(m[2])
    return out
