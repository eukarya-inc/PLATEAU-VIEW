"""``vcog.py build``: source files -> node-centred COGs + manifest.json.

Output goes to ``work/out/<key_prefix>/<version>/`` laid out exactly like
the published keys.

height-correction
    One ``<grid.file>`` per PatchJGD(H) parameter file, node for node,
    nothing merged or filled (NaN where the file has no row). The value of
    tertiary mesh M (dH at M's south-west corner) is the centre of one
    pixel, so bilinear interpolation between pixel centres is exactly
    PatchJGD(H)'s node bilinear. With a ``listed-meshes`` selection the mesh
    list is written as ``<selection.list_file>`` and the manifest carries
    the normative selection rule; the build stops if the list fetched from
    GSI differs from ``selection.expected`` (a changed list must become a
    deliberately reviewed new version).

    ``--diagnostic-merged`` also writes ``work/out/diagnostic/<id>/merged.tif``,
    the single-grid approximation (listed grid on every node a listed mesh
    reads, else primary, else fallback). It is NOT published: GSI's result
    is discontinuous along the edges of listed meshes and fallback cells, a
    node shared by both kinds of cell would need two values, and
    ``validate --oracle`` measures the resulting 1-3 cm errors.

geoid
    The full source lattice unchanged (no cropping, no resampling): pixel
    (r, c) holds the source node (r, c) counted from the north-west.
"""

from __future__ import annotations

import json
import os

import numpy as np

import vref
from products import Product

GEOID_READERS = {"isg": vref.read_isg, "gsigeo-asc": vref.read_gsigeo_asc}


def _sources(p: Product, work: str) -> dict:
    path = os.path.join(p.src_dir(work), "sources.json")
    if not os.path.exists(path):
        raise SystemExit(f"{p.id}: {path} missing; run `vcog.py fetch {p.id}` first")
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def closure_nodes(mesh6: int) -> list[tuple[int, int]]:
    """Nodes read when interpolating anywhere inside secondary mesh ``mesh6``."""
    i0, j0 = vref.mesh6_sw_node(mesh6)
    return [(i0 + a, j0 + b) for a in range(11) for b in range(11)]


def selection_rule(p: Product, files: dict[str, str]) -> dict:
    """Normative text of the listed-meshes rule, with this product's file names."""
    sel = p.get("selection")
    prim, lst, fb, lf = files[sel["primary"]], files[sel["listed"]], files[sel["fallback"]], sel["list_file"]
    return {
        "summary": (
            f"dH(p) = L(p) if p is in a listed secondary mesh; otherwise P(p) if P is in range at p; otherwise F(p). "
            f"L / P / F = the standard sample (sampler.py) of {lst} / {prim} / {fb}; the list is {lf} 'meshes'."
        ),
        "steps": [
            "1. Secondary mesh of p=(lat, lon) in degrees (JIS X 0410): i5 = floor(lat * 12), j75 = floor((lon - 100) * 8), "
            "mesh6 = (i5 div 8) * 10000 + (j75 div 8) * 100 + (i5 mod 8) * 10 + (j75 mod 8). A point exactly on a secondary-mesh "
            "boundary belongs to the mesh to its north / east (that is what floor gives).",
            f"2. If mesh6 is in {lf} 'meshes': dH = sample({lst}, p). If that is NaN the result is NaN (no fallback).",
            f"3. Otherwise vp = sample({prim}, p). If vp is not NaN, dH = vp.",
            f"4. Otherwise ('{prim} out of range at p'): dH = sample({fb}, p), NaN if that is NaN too.",
        ],
        "out_of_range": (
            f"sample({prim}, p) is NaN, i.e. ANY corner node that carries a non-zero bilinear weight at p is missing from {prim} "
            "(or outside the raster). For p strictly inside a tertiary cell that is 'any of the 4 corners missing', not 'all 4'. "
            "On a cell edge only the 2 nodes of that edge count, on a node only that node (zero-weight neighbours are not used). "
            "The fallback is therefore per point; for interior points it is exactly per tertiary cell."
        ),
        "shared_nodes": (
            "No node is ever merged. Each point is evaluated entirely on ONE grid - all four corners from the same file - chosen by "
            "steps 1-4 for that point. A node shared by cells evaluated on different grids contributes each grid's own value to "
            "its cell, so dH is discontinuous along such cell edges. This is intended: it is what GSI's DEM conversion produced."
        ),
        "sampling": "sampler.py: bilinear between pixel centres (pixel centre == node), zero-weight neighbours unused, NaN if a used neighbour is nodata/outside, 1e-9-cell snap.",
        "reference_implementation": "tile/scripts/vertical-cogs/sampler.py: DhGsi / sample_dh_gsi",
    }


def merged_diagnostic(primary, listed, fallback, meshes):
    """Single-grid approximation (not published). Returns (nodes, conflicts)."""
    listed_nodes: set[tuple[int, int]] = set()
    for m in meshes:
        listed_nodes.update(closure_nodes(m))
    merged: dict[tuple[int, int], float] = {}
    conflicts = []
    for n in set(primary) | set(listed) | set(fallback):
        if n in listed_nodes and n in listed:
            merged[n] = listed[n]
            if n in primary and primary[n] != listed[n]:
                users = {vref.node_to_mesh6(n[0] - a, n[1] - b) for a in (0, 1) for b in (0, 1)}
                if any(u not in meshes for u in users):
                    conflicts.append(abs(primary[n] - listed[n]))
        elif n in primary:
            merged[n] = primary[n]
        elif n in fallback:
            merged[n] = fallback[n]
    return merged, conflicts


def build_height_correction(p: Product, work: str, diagnostic_merged: bool = False) -> dict:
    sources = _sources(p, work)
    src = p.src_dir(work)
    grids = p.get("grid")
    sel = p.get("selection")
    pars = {g["name"]: vref.read_par(os.path.join(src, os.path.basename(g["source"]["member"]))) for g in grids}
    all_nodes = set().union(*(par.nodes for par in pars.values()))
    bbox = (min(n[0] for n in all_nodes), max(n[0] for n in all_nodes), min(n[1] for n in all_nodes), max(n[1] for n in all_nodes))
    frm, to = p.get("from"), p.get("to")
    out_dir = p.out_dir(work)
    os.makedirs(out_dir, exist_ok=True)
    files: dict[str, dict] = {}
    file_of = {g["name"]: g["file"] for g in grids}

    meshes: list[int] = []
    if sel:
        ml = sources["mesh_list"]
        meshes = sorted(ml["meshes"])
        if meshes != sorted(sel["expected"]):
            raise SystemExit(
                f"{p.id}: GSI's mesh list changed: {meshes} != expected {sorted(sel['expected'])}; "
                "review it, update products.toml and bump the version"
            )
        list_json = {
            "product": p.id,
            "version": p.version,
            "description": sel["list_description"],
            "meshes": meshes,
            "source": {
                "publisher": "国土地理院 (GSI), 基盤地図情報ダウンロードサービス",
                "page": ml["page"],
                "entry": ml["entry"],
                "quote": ml["quote"],
                "fetched_at": ml["fetched_at"],
                "page_sha256_at_fetch": ml["page_sha256"],
            },
            "note": "Not derivable from the parameter files (see README); taken verbatim from GSI's page. Validated against GSI's own JGD2024 DEM files by `vcog.py validate --oracle`.",
        }
        vref.write_json(os.path.join(out_dir, sel["list_file"]), list_json)

    common_meta = {
        "VREF_KIND": "height_correction",
        "VREF_FROM": frm,
        "VREF_TO": to,
        "VREF_UNITS": "metre",
        "VREF_SIGN": f"H_{to} = H_{frm} + value",
        "VREF_GRID": "node-centred: pixel centre = SW-corner node of a tertiary mesh (30 arcsec x 45 arcsec); sample bilinearly between pixel centres (sampler.py)",
        "VREF_NODATA": "NaN",
    }
    if sel:
        other = "the other grid" if len(grids) == 2 else "the other grids"
        common_meta["VREF_SELECTION"] = f"use with {other} and {sel['list_file']} per manifest.json selection_rule; never use one grid alone"

    grid_info: dict = {}
    for g in grids:
        par = pars[g["name"]]
        ng = vref.mesh_nodes_to_grid_bbox(par.nodes, bbox)
        path = os.path.join(out_dir, g["file"])
        meta = dict(common_meta, VREF_CONTENT=g["content"], VREF_NODE_ORIGIN=f"lat_n={ng.lat_n!r} lon_w={ng.lon_w!r} dlat={ng.dlat!r} dlon={ng.dlon!r}")
        vref.write_cog(ng, path, meta, f"{frm.upper()}->{to.upper()} height correction dH (m): {g['content']}")
        files[g["file"]] = {
            "content": g["content"],
            "par_version": par.version,
            "valid_nodes": int((~np.isnan(ng.data)).sum()),
            "sha256": vref.sha256_file(path),
            "bytes": os.path.getsize(path),
        }
        grid_info = {
            "convention": "node-centred (pixel centre == SW-corner node of the tertiary mesh; PixelIsArea geotransform offset by half a cell)",
            "rows": ng.rows,
            "cols": ng.cols,
            "nw_node_lat": ng.lat_n,
            "nw_node_lon": ng.lon_w,
            "dlat_deg": ng.dlat,
            "dlon_deg": ng.dlon,
            "geotransform": list(ng.geotransform()),
            "note": "all grids of this product share this exact grid",
        }
        print(path, files[g["file"]])

    manifest: dict = {
        "kind": "height_correction",
        "product": p.id,
        "version": p.version,
        "from": frm,
        "to": to,
        "units": "m",
        "sign": f"H_{to} = H_{frm} + dH",
        "crs": "EPSG:6668",
        "nodata": "NaN",
        "dtype": "float32",
        "grid": grid_info,
        "files": files,
    }
    if sel:
        lf = os.path.join(out_dir, sel["list_file"])
        files[sel["list_file"]] = {"content": "secondary-mesh list for the selection rule", "sha256": vref.sha256_file(lf), "bytes": os.path.getsize(lf)}
        manifest["selection"] = {
            "kind": sel["kind"],
            "primary": file_of[sel["primary"]],
            "listed": file_of[sel["listed"]],
            "fallback": file_of[sel["fallback"]],
            "list": sel["list_file"],
        }
        manifest["selection_rule"] = selection_rule(p, file_of)
        manifest["listed_meshes"] = meshes
        lst = pars[sel["listed"]].nodes
        manifest["listed_mesh_nodes_missing_from_listed_grid"] = sorted(
            vref.node_to_mesh8(*n) for m in meshes for n in closure_nodes(m) if n not in lst
        )
    else:
        manifest["selection_rule"] = {"summary": f"dH(p) = sample({grids[0]['file']}, p) (sampler.py)"}
    manifest["sources"] = sources
    manifest["validation"] = "vcog.py validate --oracle: 2011 + dH vs GSI's JGD2024 DEM re-issue; see README"
    vref.write_json(os.path.join(out_dir, "manifest.json"), manifest)

    if diagnostic_merged and sel:
        merged, conflicts = merged_diagnostic(pars[sel["primary"]].nodes, pars[sel["listed"]].nodes, pars[sel["fallback"]].nodes, set(meshes))
        ng = vref.mesh_nodes_to_grid_bbox(merged, bbox)
        path = os.path.join(work, "out", "diagnostic", p.id, "merged.tif")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        vref.write_cog(ng, path, dict(common_meta, VREF_CONTENT="DIAGNOSTIC ONLY - single-grid approximation, not published"), "diagnostic merged dH (not published)")
        print(path, f"conflict nodes {len(conflicts)}, max |primary-listed| {max(conflicts, default=0):.5f} m")
    return manifest


def build_geoid(p: Product, work: str) -> dict:
    sources = _sources(p, work)
    src_meta = sources["geoid"]
    src_path = os.path.join(p.src_dir(work), src_meta["file"])
    gs = GEOID_READERS[p.get("format")](src_path)
    g = gs.grid
    out_dir = p.out_dir(work)
    os.makedirs(out_dir, exist_ok=True)
    tif = os.path.join(out_dir, "geoid.tif")
    valid = ~np.isnan(g.data)
    meta = {
        "VREF_KIND": "geoid",
        "VREF_MODEL": p.id,
        "VREF_UNITS": "metre",
        "VREF_SIGN": "ellipsoidal_height = orthometric_height + value",
        "VREF_GRID": "node-centred: pixel centres are the source grid nodes; sample bilinearly between pixel centres (sampler.py)",
        "VREF_NODE_ORIGIN": f"lat_n={g.lat_n!r} lon_w={g.lon_w!r} dlat={g.dlat!r} dlon={g.dlon!r}",
        "VREF_NODATA": "NaN (source nodata -9999 / 999)",
        "VREF_SOURCE_FILE": src_meta["file"],
        "VREF_SOURCE_SHA256": vref.sha256_file(src_path),
    }
    vref.write_cog(g, tif, meta, f"{p.id} geoid height (m), node-centred grid")
    manifest = {
        "kind": "geoid",
        "model": p.id,
        "version": p.version,
        "model_name_in_source": gs.model,
        "vertical_datum": p.get("vertical_datum"),
        "note": p.get("note"),
        "units": "m",
        "sign": "ellipsoidal_height = orthometric_height + value",
        "crs": "EPSG:6668",
        "nodata": "NaN",
        "dtype": "float32",
        "grid": {
            "convention": "node-centred (pixel centre == source node; PixelIsArea geotransform offset by half a cell)",
            "rows": g.rows,
            "cols": g.cols,
            "nw_node_lat": g.lat_n,
            "nw_node_lon": g.lon_w,
            "dlat_deg": g.dlat,
            "dlon_deg": g.dlon,
            "geotransform": list(g.geotransform()),
            "valid_nodes": int(valid.sum()),
        },
        "sampling": "bilinear on pixel centres, NaN if any used neighbour is nodata/outside; see sampler.py",
        "source": {"header": gs.header, **src_meta},
        "files": {"geoid.tif": {"sha256": vref.sha256_file(tif), "bytes": os.path.getsize(tif)}},
    }
    vref.write_json(os.path.join(out_dir, "manifest.json"), manifest)
    print(tif, manifest["files"]["geoid.tif"], f"valid nodes {manifest['grid']['valid_nodes']}")
    return manifest


def build(p: Product, work: str, diagnostic_merged: bool = False) -> dict:
    if p.kind == "height-correction":
        return build_height_correction(p, work, diagnostic_merged)
    return build_geoid(p, work)
