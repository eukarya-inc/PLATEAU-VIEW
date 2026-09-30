# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy"]
# ///
"""Geoid grid (GSI ISG 2.0 / GSIGEO ASC) -> node-centred float32 COG.

    uv run build_geoid.py jpgeo2024-hrefconv2024 --src work/src --out work/out
    uv run build_geoid.py gsigeo2011-v2.2        --src work/src --out work/out

The full source lattice is written unchanged (no cropping, no resampling):
pixel (r, c) holds exactly the source value of node (r, c) counted from the
north-west, and the geotransform puts that pixel's centre on the node.
"""

from __future__ import annotations

import argparse
import json
import os

import numpy as np

import vref

MODELS = {
    "jpgeo2024-hrefconv2024": {
        "file": "JPGEO2024+Hrefconv2024.isg",
        "reader": vref.read_isg,
        "source_key": "jpgeo2024_isg",
        "vertical_datum": "JGD2024 (標高 = 楕円体高 - ジオイド高; includes Hrefconv2024 island offsets)",
        "note": "GSI's combined JPGEO2024 + Hrefconv2024 grid as published in JPGEO2024_isg.zip; GSI Q&A A2-15 permits it nationwide. Not re-derived.",
    },
    "gsigeo2011-v2.2": {
        "file": "gsigeo2011_ver2_2.asc",
        "reader": vref.read_gsigeo_asc,
        "source_key": "gsigeo2011",
        "vertical_datum": "JGD2011 (日本のジオイド2011 Ver.2.2)",
        "note": "Kept for parity with the existing JGD2011 terrain dataset.",
    },
}


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("model", choices=sorted(MODELS))
    ap.add_argument("--src", default="work/src")
    ap.add_argument("--out", default="work/out")
    ap.add_argument("--version", default="v1")
    args = ap.parse_args()
    m = MODELS[args.model]
    src_path = os.path.join(args.src, m["file"])
    gs = m["reader"](src_path)
    g = gs.grid
    sources = json.load(open(os.path.join(args.src, "sources.json"), encoding="utf-8"))

    out_dir = os.path.join(args.out, "geoid", args.model)
    os.makedirs(out_dir, exist_ok=True)
    tif = os.path.join(out_dir, "geoid.tif")
    valid = ~np.isnan(g.data)
    meta = {
        "VREF_KIND": "geoid",
        "VREF_MODEL": args.model,
        "VREF_UNITS": "metre",
        "VREF_SIGN": "ellipsoidal_height = orthometric_height + value",
        "VREF_GRID": "node-centred: pixel centres are the source grid nodes; sample bilinearly between pixel centres (sampler.py)",
        "VREF_NODE_ORIGIN": f"lat_n={g.lat_n!r} lon_w={g.lon_w!r} dlat={g.dlat!r} dlon={g.dlon!r}",
        "VREF_NODATA": "NaN (source nodata -9999 / 999)",
        "VREF_SOURCE_FILE": m["file"],
        "VREF_SOURCE_SHA256": vref.sha256_file(src_path),
    }
    vref.write_cog(g, tif, meta, f"{args.model} geoid height (m), node-centred grid")
    manifest = {
        "kind": "geoid",
        "model": args.model,
        "version": args.version,
        "model_name_in_source": gs.model,
        "vertical_datum": m["vertical_datum"],
        "note": m["note"],
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
        "source": {"header": gs.header, **sources.get(m["source_key"], {})},
        "cog": {"file": "geoid.tif", "sha256": vref.sha256_file(tif), "bytes": os.path.getsize(tif)},
    }
    vref.write_json(os.path.join(out_dir, "manifest.json"), manifest)
    print(tif, manifest["cog"], f"valid nodes {manifest['grid']['valid_nodes']}")


if __name__ == "__main__":
    main()
