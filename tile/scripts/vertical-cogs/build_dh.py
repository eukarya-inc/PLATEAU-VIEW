# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy"]
# ///
"""PatchJGD(H) hyokorevBM/TR (JGD2011 -> JGD2024 height correction) -> COGs.

    uv run build_dh.py --src work/src --out work/out

Writes, under ``<out>/hyokorev-jgd2011-to-jgd2024/``:

* ``dh_bm.tif`` / ``dh_tr.tif`` -- each parameter file on its own, node for
  node, nothing merged or filled (NaN where the file has no row).
* ``dh.tif`` -- the single merged grid (see MERGE RULE below).
* ``manifest.json`` for all three.

Every raster is node-centred: the value of tertiary mesh M (dH at M's
south-west corner) is the centre of one pixel, so bilinear interpolation
between pixel centres is exactly PatchJGD(H)'s node bilinear. Sign:
``H_jgd2024 = H_jgd2011 + dH``.

MERGE RULE for dh.tif
---------------------
GSI converted its DEMs per secondary mesh with *one* parameter file: BM,
or TR for the secondary meshes listed in ``GSI_TR_FALLBACK`` below. Within a
secondary mesh the interpolation of every point uses the 4 corner nodes of
its tertiary cell, so a secondary mesh reads the 11 x 11 nodes of its own
10 x 10 cells plus the first row/column of its north/east neighbours.

A single node grid cannot give a node shared by a BM mesh and a TR mesh two
values, so dh.tif is the following compromise (its error is quantified by
validate_oracle.py and ``conflict_nodes`` in the manifest):

1. every node read by a TR-fallback secondary mesh takes TR -- so the listed
   meshes reproduce GSI exactly;
2. otherwise BM where BM has the node;
3. otherwise TR where TR has the node (sea margins, BM-gap areas outside
   GSI's DEM coverage);
4. otherwise NaN (Northern Territories, Iwo-to, Senkaku, open sea, ...).

Nodes where rule 1 overrides an existing, different BM value are
"conflict nodes": GSI used BM there for the neighbouring BM meshes, so
dh.tif differs from GSI in the one-cell strip of those neighbours.
"""

from __future__ import annotations

import argparse
import json
import os

import numpy as np

import vref

# https://service.gsi.go.jp/kiban/app/data_update_info_all/  (2025-07-31
# 「提供データを整備・更新しました（数値標高モデル）」): secondary meshes whose
# 2025-07 DEM re-issue used hyokorevTR_jgd2024_h.par instead of hyokorevBM.
GSI_TR_FALLBACK = sorted(
    {
        473113, 473121, 473122, 473123, 473131, 473132, 473133, 473141, 473142, 473143,
        473151, 473152, 473153, 473161, 473162, 473163, 473173,
        543664, 543665, 543674, 543675, 553605,
    }
)
GSI_TR_FALLBACK_SOURCE = "https://service.gsi.go.jp/kiban/app/data_update_info_all/ (2025-07-31 entry)"


def closure_nodes(mesh6: int) -> list[tuple[int, int]]:
    """Nodes read when interpolating anywhere inside secondary mesh ``mesh6``."""
    i0, j0 = vref.mesh6_sw_node(mesh6)
    return [(i0 + a, j0 + b) for a in range(11) for b in range(11)]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", default="work/src")
    ap.add_argument("--out", default="work/out")
    args = ap.parse_args()
    sources = json.load(open(os.path.join(args.src, "sources.json"), encoding="utf-8"))
    bm = vref.read_par(os.path.join(args.src, "hyokorevBM_jgd2024_h.par"))
    tr = vref.read_par(os.path.join(args.src, "hyokorevTR_jgd2024_h.par"))
    B, T = bm.nodes, tr.nodes

    tr_nodes: set[tuple[int, int]] = set()
    for m in GSI_TR_FALLBACK:
        tr_nodes.update(closure_nodes(m))

    merged: dict[tuple[int, int], float] = {}
    origin: dict[str, int] = {"tr_fallback": 0, "bm": 0, "tr_fill": 0}
    conflicts: list[dict] = []
    for n in set(B) | set(T):
        if n in tr_nodes and n in T:
            merged[n] = T[n]
            origin["tr_fallback"] += 1
            if n in B and B[n] != T[n]:
                # is this node also read by a BM secondary mesh?
                users = {vref.node_to_mesh6(n[0] - a, n[1] - b) for a in (0, 1) for b in (0, 1)}
                bm_users = sorted(u for u in users if u not in GSI_TR_FALLBACK)
                if bm_users:
                    conflicts.append(
                        {"mesh8": vref.node_to_mesh8(*n), "bm": B[n], "tr": T[n], "abs_diff": round(abs(B[n] - T[n]), 5), "bm_meshes_reading_it": bm_users}
                    )
        elif n in B:
            merged[n] = B[n]
            origin["bm"] += 1
        elif n in T:
            merged[n] = T[n]
            origin["tr_fill"] += 1
    missing_tr = sorted(vref.node_to_mesh8(*n) for n in tr_nodes if n not in T and n in B)

    ii = [n[0] for n in (set(B) | set(T))]
    jj = [n[1] for n in (set(B) | set(T))]
    bbox = (min(ii), max(ii), min(jj), max(jj))

    out_dir = os.path.join(args.out, "hyokorev-jgd2011-to-jgd2024")
    os.makedirs(out_dir, exist_ok=True)
    common_meta = {
        "VREF_KIND": "height_correction",
        "VREF_FROM": "jgd2011",
        "VREF_TO": "jgd2024",
        "VREF_UNITS": "metre",
        "VREF_SIGN": "H_jgd2024 = H_jgd2011 + value",
        "VREF_GRID": "node-centred: pixel centre = SW-corner node of a tertiary mesh (30\" x 45\"); sample bilinearly between pixel centres (sampler.py)",
        "VREF_NODATA": "NaN",
    }
    files = {}
    for name, nodes, desc in (
        ("dh_bm.tif", B, "hyokorevBM_jgd2024_h.par as-is"),
        ("dh_tr.tif", T, "hyokorevTR_jgd2024_h.par as-is"),
        ("dh.tif", merged, "BM, with TR for GSI's TR-fallback secondary meshes and as gap fill (see manifest merge_rule)"),
    ):
        g = vref.mesh_nodes_to_grid_bbox(nodes, bbox)
        path = os.path.join(out_dir, name)
        meta = dict(common_meta, VREF_CONTENT=desc, VREF_NODE_ORIGIN=f"lat_n={g.lat_n!r} lon_w={g.lon_w!r} dlat={g.dlat!r} dlon={g.dlon!r}")
        vref.write_cog(g, path, meta, f"JGD2011->JGD2024 height correction dH (m): {desc}")
        files[name] = {
            "content": desc,
            "valid_nodes": int((~np.isnan(g.data)).sum()),
            "sha256": vref.sha256_file(path),
            "bytes": os.path.getsize(path),
        }
        grid_info = {
            "convention": "node-centred (pixel centre == SW-corner node of the tertiary mesh; PixelIsArea geotransform offset by half a cell)",
            "rows": g.rows,
            "cols": g.cols,
            "nw_node_lat": g.lat_n,
            "nw_node_lon": g.lon_w,
            "dlat_deg": g.dlat,
            "dlon_deg": g.dlon,
            "geotransform": list(g.geotransform()),
        }
        print(path, files[name])

    manifest = {
        "kind": "height_correction",
        "from": "jgd2011",
        "to": "jgd2024",
        "units": "m",
        "sign": "H_jgd2024 = H_jgd2011 + dH",
        "crs": "EPSG:6668",
        "nodata": "NaN",
        "dtype": "float32",
        "grid": grid_info,
        "sampling": "bilinear on pixel centres, NaN if any used neighbour is nodata/outside; see sampler.py",
        "files": files,
        "merge_rule": {
            "summary": __doc__.split("MERGE RULE for dh.tif")[1].split("---------------------")[1].strip(),
            "tr_fallback_secondary_meshes": GSI_TR_FALLBACK,
            "tr_fallback_source": GSI_TR_FALLBACK_SOURCE,
            "node_origin_counts": origin,
            "tr_fallback_nodes_without_tr": missing_tr,
            "conflict_nodes": len(conflicts),
            "conflict_max_abs_diff_m": max((c["abs_diff"] for c in conflicts), default=0.0),
            "conflicts": sorted(conflicts, key=lambda c: -c["abs_diff"]),
        },
        "sources": {k: sources[k] for k in ("hyokorevBM", "hyokorevTR")},
        "par_versions": {"BM": bm.version, "TR": tr.version},
    }
    vref.write_json(os.path.join(out_dir, "manifest.json"), manifest)
    print("node origins", origin, "conflict nodes", len(conflicts), "max", manifest["merge_rule"]["conflict_max_abs_diff_m"])


if __name__ == "__main__":
    main()
