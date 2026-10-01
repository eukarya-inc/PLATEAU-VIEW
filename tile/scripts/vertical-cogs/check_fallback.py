"""Compare a product's published mesh list with what its parameter files imply.

Part of ``vcog.py validate`` for height-correction products with a
``listed-meshes`` selection.

Candidate rule tested: a secondary mesh "contains a gap in the primary grid"
when one of the nodes its own cells interpolate from (its 11 x 11 closure:
own 10 x 10 SW nodes plus the first row/column of the N/E neighbours) is
absent from the primary grid but present in the listed grid. Each candidate
not on the list can be checked against GSI's DEM catalogue (does GSI publish
any DEM1A/5A/5B/5C there at all?). For hyokorev this shows the list is not
derivable from the parameter files (see README).
"""

from __future__ import annotations

import os

import gsi
import vref
from build import closure_nodes
from products import Product


def check(p: Product, work: str, catalogue: bool = False) -> dict:
    sel = p.get("selection")
    by_name = {g["name"]: g for g in p.get("grid")}
    src = p.src_dir(work)

    def nodes(role: str):
        return vref.read_par(os.path.join(src, os.path.basename(by_name[sel[role]]["source"]["member"]))).nodes

    P, L = nodes("primary"), nodes("listed")
    secs = {vref.node_to_mesh6(*n) for n in set(P) | set(L)}

    def gap_cells(m: int) -> list[int]:
        i0, j0 = vref.mesh6_sw_node(m)
        out = []
        for a in range(10):
            for b in range(10):
                c = [(i0 + a + da, j0 + b + db) for da in (0, 1) for db in (0, 1)]
                if any(n not in P for n in c) and all(n in L for n in c):
                    out.append(vref.node_to_mesh8(i0 + a, j0 + b))
        return out

    cand = sorted(m for m in secs if any(n not in P and n in L for n in closure_nodes(m)))
    listed = set(sel["expected"])
    res: dict = {
        "rule": "closure node missing in primary and present in listed grid",
        "list": sorted(listed),
        "candidates": len(cand),
        "list_missing_from_candidates": sorted(listed - set(cand)),
        "extra_candidates": {},
    }
    cl = gsi.Client() if catalogue else None
    for m in sorted(set(cand) - listed):
        e: dict = {"gap_cells": gap_cells(m)}
        if cl:
            e["dem_types"] = sorted({x["type_code"] for x in cl.dem_editions(str(m))})
        res["extra_candidates"][str(m)] = e
    return res
