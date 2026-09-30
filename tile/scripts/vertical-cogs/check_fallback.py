# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy"]
# ///
"""Compare GSI's published TR-fallback list with what the parameter files imply.

    uv run check_fallback.py --src work/src [--no-catalogue]

Candidate rule tested: a secondary mesh "contains a BM gap" when one of the
nodes its own cells interpolate from (its 11 x 11 closure: own 10 x 10 SW
nodes plus the first row/column of the N/E neighbours) is absent from BM
but present in TR. Each candidate outside GSI's list is then checked against
the DEM catalogue (does GSI publish any DEM1A/5A/5B/5C there at all?).
"""

from __future__ import annotations

import argparse
import json
import os

import gsi
import vref
from build_dh import GSI_TR_FALLBACK, closure_nodes


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", default="work/src")
    ap.add_argument("--no-catalogue", action="store_true")
    args = ap.parse_args()
    B = vref.read_par(os.path.join(args.src, "hyokorevBM_jgd2024_h.par")).nodes
    T = vref.read_par(os.path.join(args.src, "hyokorevTR_jgd2024_h.par")).nodes
    secs = {vref.node_to_mesh6(*n) for n in set(B) | set(T)}

    def gap_cells(m: int) -> list[int]:
        i0, j0 = vref.mesh6_sw_node(m)
        out = []
        for a in range(10):
            for b in range(10):
                c = [(i0 + a + da, j0 + b + db) for da in (0, 1) for db in (0, 1)]
                if any(n not in B for n in c) and all(n in T for n in c):
                    out.append(vref.node_to_mesh8(i0 + a, j0 + b))
        return out

    cand = sorted(m for m in secs if any(n not in B and n in T for n in closure_nodes(m)))
    gsi_set = set(GSI_TR_FALLBACK)
    res = {
        "rule": "closure node missing in BM and present in TR",
        "gsi_list": GSI_TR_FALLBACK,
        "candidates": len(cand),
        "gsi_list_missing_from_candidates": sorted(gsi_set - set(cand)),
        "extra_candidates": {},
    }
    cl = gsi.Client()
    for m in sorted(set(cand) - gsi_set):
        e: dict = {"gap_cells": gap_cells(m)}
        if not args.no_catalogue:
            eds = cl.dem_editions(str(m))
            e["dem_types"] = sorted({x["type_code"] for x in eds})
        res["extra_candidates"][str(m)] = e
    print(json.dumps(res, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
