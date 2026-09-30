# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy"]
# ///
"""PatchJGD(H) hyokorevBM/TR (JGD2011 -> JGD2024 height correction) -> COGs.

    uv run build_dh.py --src work/src --out work/out [--diagnostic-merged]

Writes the served product under ``<out>/hyokorev-jgd2011-to-jgd2024/``:

* ``dh_bm.tif`` / ``dh_tr.tif`` -- each parameter file on its own, node for
  node, nothing merged or filled (NaN where the file has no row).
* ``tr_meshes.json`` -- GSI's list of secondary meshes converted with TR,
  with provenance (taken from ``sources.json``, i.e. fetched from GSI's page).
* ``manifest.json`` -- grids, provenance and the normative SELECTION RULE.

Every raster is node-centred: the value of tertiary mesh M (dH at M's
south-west corner) is the centre of one pixel, so bilinear interpolation
between pixel centres is exactly PatchJGD(H)'s node bilinear. Sign:
``H_jgd2024 = H_jgd2011 + dH``.

``--diagnostic-merged`` additionally writes ``dh_merged_diagnostic.tif``: the
single-grid approximation (TR on every node a listed mesh reads, else BM,
else TR). It is NOT served: GSI's result is discontinuous along the edges of
TR meshes and TR fallback cells, a node shared by a BM cell and a TR cell
would need two values, and validate_oracle.py measures the resulting 1-3 cm
errors in the neighbouring BM meshes. It exists only to keep that measurable.
"""

from __future__ import annotations

import argparse
import json
import os

import numpy as np

import vref

PRODUCT = "hyokorev-jgd2011-to-jgd2024"

# The list as read on 2026-10-01. fetch_sources.py re-reads it from GSI's
# page every run; if the page ever says something else the build stops, so a
# changed list becomes a deliberate new version rather than a silent change.
EXPECTED_TR_MESHES = [
    473113, 473121, 473122, 473123, 473131, 473132, 473133, 473141, 473142, 473143,
    473151, 473152, 473153, 473161, 473162, 473163, 473173,
    543664, 543665, 543674, 543675, 553605,
]

SELECTION_RULE = {
    "summary": (
        "dH(p) = TR(p) if p is in a listed secondary mesh; otherwise BM(p) if BM is in range at p; "
        "otherwise TR(p). BM(p) / TR(p) = the standard sample (sampler.py) of dh_bm.tif / dh_tr.tif."
    ),
    "steps": [
        "1. Secondary mesh of p=(lat, lon) in degrees (JIS X 0410): i5 = floor(lat * 12), j75 = floor((lon - 100) * 8), "
        "mesh6 = (i5 div 8) * 10000 + (j75 div 8) * 100 + (i5 mod 8) * 10 + (j75 mod 8). A point exactly on a secondary-mesh "
        "boundary belongs to the mesh to its north / east (that is what floor gives).",
        "2. If mesh6 is in tr_meshes.json 'meshes': dH = sample(dh_tr.tif, p). If that is NaN the result is NaN (no fallback to BM).",
        "3. Otherwise vb = sample(dh_bm.tif, p). If vb is not NaN, dH = vb.",
        "4. Otherwise ('BM out of range at p'): dH = sample(dh_tr.tif, p), NaN if that is NaN too.",
    ],
    "bm_out_of_range": (
        "sample(dh_bm.tif, p) is NaN, i.e. ANY corner node that carries a non-zero bilinear weight at p is missing from BM "
        "(or outside the raster). For p strictly inside a tertiary cell that is 'any of the 4 corners missing', not 'all 4'. "
        "On a cell edge only the 2 nodes of that edge count, on a node only that node (zero-weight neighbours are not used). "
        "The fallback is therefore per point; for interior points it is exactly per tertiary cell, which is what GSI did "
        "(483103: only cell 48310309, whose SE corner 48310400 is absent from BM, was converted with TR)."
    ),
    "shared_nodes": (
        "No node is ever merged. Each point is evaluated entirely on ONE grid - all four corners from dh_bm.tif or all four "
        "from dh_tr.tif - chosen by steps 1-4 for that point. A node shared by a BM-evaluated cell and a TR-evaluated cell "
        "contributes its BM value to the first and its TR value to the second, so dH is discontinuous along such cell edges "
        "(e.g. the 473120 | 473121 boundary). This is intended: it is what GSI's DEM conversion produced."
    ),
    "sampling": "sampler.py: bilinear between pixel centres (pixel centre == node), zero-weight neighbours unused, NaN if a used neighbour is nodata/outside, 1e-9-cell snap.",
    "reference_implementation": "tile/scripts/vertical-cogs/sampler.py: DhGsi / sample_dh_gsi",
}


def closure_nodes(mesh6: int) -> list[tuple[int, int]]:
    """Nodes read when interpolating anywhere inside secondary mesh ``mesh6``."""
    i0, j0 = vref.mesh6_sw_node(mesh6)
    return [(i0 + a, j0 + b) for a in range(11) for b in range(11)]


def merged_diagnostic(B, T, tr_meshes):
    """Single-grid approximation (not served). Returns (nodes, conflicts)."""
    tr_nodes: set[tuple[int, int]] = set()
    for m in tr_meshes:
        tr_nodes.update(closure_nodes(m))
    merged: dict[tuple[int, int], float] = {}
    conflicts = []
    for n in set(B) | set(T):
        if n in tr_nodes and n in T:
            merged[n] = T[n]
            if n in B and B[n] != T[n]:
                users = {vref.node_to_mesh6(n[0] - a, n[1] - b) for a in (0, 1) for b in (0, 1)}
                if any(u not in tr_meshes for u in users):
                    conflicts.append(abs(B[n] - T[n]))
        elif n in B:
            merged[n] = B[n]
        elif n in T:
            merged[n] = T[n]
    return merged, conflicts


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", default="work/src")
    ap.add_argument("--out", default="work/out")
    ap.add_argument("--version", default="v1")
    ap.add_argument("--diagnostic-merged", action="store_true")
    args = ap.parse_args()
    sources = json.load(open(os.path.join(args.src, "sources.json"), encoding="utf-8"))
    tr_src = sources["tr_list"]
    tr_meshes = sorted(tr_src["meshes"])
    if tr_meshes != EXPECTED_TR_MESHES:
        raise SystemExit(f"GSI's TR list changed: {tr_meshes} != {EXPECTED_TR_MESHES}; review and bump the version")
    bm = vref.read_par(os.path.join(args.src, "hyokorevBM_jgd2024_h.par"))
    tr = vref.read_par(os.path.join(args.src, "hyokorevTR_jgd2024_h.par"))
    B, T = bm.nodes, tr.nodes
    all_nodes = set(B) | set(T)
    bbox = (min(n[0] for n in all_nodes), max(n[0] for n in all_nodes), min(n[1] for n in all_nodes), max(n[1] for n in all_nodes))
    listed_nodes_without_tr = sorted(vref.node_to_mesh8(*n) for m in tr_meshes for n in closure_nodes(m) if n not in T)

    out_dir = os.path.join(args.out, PRODUCT)
    os.makedirs(out_dir, exist_ok=True)

    tr_json = {
        "product": PRODUCT,
        "version": args.version,
        "description": "JIS X 0410 secondary meshes (6-digit codes) whose JGD2024 DEM re-issue GSI computed with hyokorevTR_jgd2024_h.par instead of hyokorevBM_jgd2024_h.par. Used by step 2 of the selection rule in manifest.json.",
        "meshes": tr_meshes,
        "source": {
            "publisher": "国土地理院 (GSI), 基盤地図情報ダウンロードサービス",
            "page": tr_src["page"],
            "entry": tr_src["entry"],
            "quote": tr_src["quote"],
            "fetched_at": tr_src["fetched_at"],
            "page_sha256_at_fetch": tr_src["page_sha256"],
        },
        "note": "Not derivable from the parameter files (see README); taken verbatim from GSI's page. Validated against GSI's own JGD2024 DEM files by validate_oracle.py.",
    }
    tr_path = os.path.join(out_dir, "tr_meshes.json")
    vref.write_json(tr_path, tr_json)

    common_meta = {
        "VREF_KIND": "height_correction",
        "VREF_FROM": "jgd2011",
        "VREF_TO": "jgd2024",
        "VREF_UNITS": "metre",
        "VREF_SIGN": "H_jgd2024 = H_jgd2011 + value",
        "VREF_GRID": "node-centred: pixel centre = SW-corner node of a tertiary mesh (30 arcsec x 45 arcsec); sample bilinearly between pixel centres (sampler.py)",
        "VREF_NODATA": "NaN",
        "VREF_SELECTION": "use with the other grid and tr_meshes.json per manifest.json selection_rule; never use one grid alone",
    }
    files: dict[str, dict] = {}
    grid_info: dict = {}
    for name, par, desc in (
        ("dh_bm.tif", bm, "hyokorevBM_jgd2024_h.par as-is (水準点標高補正)"),
        ("dh_tr.tif", tr, "hyokorevTR_jgd2024_h.par as-is (三角点標高補正)"),
    ):
        g = vref.mesh_nodes_to_grid_bbox(par.nodes, bbox)
        path = os.path.join(out_dir, name)
        meta = dict(common_meta, VREF_CONTENT=desc, VREF_NODE_ORIGIN=f"lat_n={g.lat_n!r} lon_w={g.lon_w!r} dlat={g.dlat!r} dlon={g.dlon!r}")
        vref.write_cog(g, path, meta, f"JGD2011->JGD2024 height correction dH (m): {desc}")
        files[name] = {"content": desc, "par_version": par.version, "valid_nodes": int((~np.isnan(g.data)).sum()), "sha256": vref.sha256_file(path), "bytes": os.path.getsize(path)}
        grid_info = {
            "convention": "node-centred (pixel centre == SW-corner node of the tertiary mesh; PixelIsArea geotransform offset by half a cell)",
            "rows": g.rows,
            "cols": g.cols,
            "nw_node_lat": g.lat_n,
            "nw_node_lon": g.lon_w,
            "dlat_deg": g.dlat,
            "dlon_deg": g.dlon,
            "geotransform": list(g.geotransform()),
            "note": "dh_bm.tif and dh_tr.tif share this exact grid",
        }
        print(path, files[name])
    files["tr_meshes.json"] = {"content": "GSI's TR secondary-mesh list", "sha256": vref.sha256_file(tr_path), "bytes": os.path.getsize(tr_path)}

    manifest = {
        "kind": "height_correction",
        "product": PRODUCT,
        "version": args.version,
        "from": "jgd2011",
        "to": "jgd2024",
        "units": "m",
        "sign": "H_jgd2024 = H_jgd2011 + dH",
        "crs": "EPSG:6668",
        "nodata": "NaN",
        "dtype": "float32",
        "grid": grid_info,
        "files": files,
        "selection_rule": SELECTION_RULE,
        "tr_meshes": tr_meshes,
        "listed_mesh_nodes_without_tr": listed_nodes_without_tr,
        "sources": {k: sources[k] for k in ("hyokorevBM", "hyokorevTR", "tr_list")},
        "validation": "validate_oracle.py: 2011 + dH vs GSI's JGD2024 DEM re-issue on 473121, 473120, 483103, 543664 (DEM5A), 574037 (DEM1A), 664241 (DEM5A); see README",
    }
    vref.write_json(os.path.join(out_dir, "manifest.json"), manifest)

    if args.diagnostic_merged:
        merged, conflicts = merged_diagnostic(B, T, set(tr_meshes))
        g = vref.mesh_nodes_to_grid_bbox(merged, bbox)
        path = os.path.join(args.out, "diagnostic", "dh_merged_diagnostic.tif")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        vref.write_cog(g, path, dict(common_meta, VREF_CONTENT="DIAGNOSTIC ONLY - single-grid approximation, not served"), "diagnostic merged dH (not served)")
        print(path, f"conflict nodes {len(conflicts)}, max |BM-TR| {max(conflicts, default=0):.5f} m")


if __name__ == "__main__":
    main()
