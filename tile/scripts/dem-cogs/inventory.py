"""``demcog.py inventory``: datum label of every grid in the backup, per primary mesh.

Opens the header of every per-grid GeoTIFF in the backup (``/vsis3/``; the
FME conversion wrote the FG-GML ``srsName`` as EPSG:4612 for ``jgd2000.bl``
and EPSG:6668 for ``jgd2011.bl``) and writes

* ``work/inventory/<product>.jsonl`` -- name, mesh, product, edition, bytes, md5, epsg (resumable cache)
* ``work/inventory/summary.json``    -- per product label counts, primaries whose grids
  carry more than one label (the ones the old pipeline silently truncated), unreadable files
* ``work/inventory/labels.tsv``      -- one row per (secondary mesh, product): editions and labels

This is what makes "which datum label does mesh X carry" answerable for the
served stack without downloading anything again.
"""

from __future__ import annotations

import collections
import json
import os

import meshcode
import sources


def run(cfg, args, products: list[str]) -> dict:
    root = sources.use_backup(cfg.defaults, args.rclone_config)
    tree = sources.SourceTree(root, os.path.join(args.work, "listings"))
    inv_dir = os.path.join(args.work, "inventory")
    rows: list[dict] = []
    for p in products:
        files = tree.files(p)
        print(f"{p}: {len(files)} grids, scanning headers ...", flush=True)
        epsg = sources.scan_epsg(files, os.path.join(inv_dir, f"{p.lower()}.jsonl"), threads=args.threads)
        for f in files:
            rows.append({"name": f.name, "mesh": f.grid.mesh, "product": p, "edition": f.grid.edition, "epsg": epsg[f.name]})
    summary = summarize(rows, cfg)
    with open(os.path.join(inv_dir, "summary.json"), "w") as f:
        json.dump(summary, f, indent=1, ensure_ascii=False)
    write_labels_tsv(rows, os.path.join(inv_dir, "labels.tsv"))
    for p, c in summary["labels"].items():
        print(f"{p}: {c}")
    print(f"primaries with more than one label: {len(summary['mixed_primaries'])}; unreadable: {len(summary['unreadable'])}")
    return summary


def summarize(rows: list[dict], cfg) -> dict:
    labels: dict[str, collections.Counter] = collections.defaultdict(collections.Counter)
    per_primary: dict[tuple[str, str], collections.Counter] = collections.defaultdict(collections.Counter)
    unreadable = []
    for r in rows:
        e = r["epsg"]
        lab = meshcode.EPSG_LABEL.get(e, f"EPSG:{e}")
        labels[r["product"]][lab] += 1
        if e is None:
            unreadable.append(r["name"])
        stack = cfg.stack_of_product(r["product"]).id
        per_primary[(stack, r["mesh"][:4])][lab] += 1
    mixed = {f"{s}/{p}": dict(c) for (s, p), c in sorted(per_primary.items()) if len(c) > 1}
    return {"labels": {p: dict(c) for p, c in labels.items()}, "mixed_primaries": mixed, "unreadable": unreadable}


def write_labels_tsv(rows: list[dict], path: str) -> None:
    by: dict[tuple[str, str], list[dict]] = collections.defaultdict(list)
    for r in rows:
        by[(r["mesh"][:6], r["product"])].append(r)
    with open(path, "w") as f:
        f.write("mesh6\tproduct\tgrids\teditions\tlabels\n")
        for (m, p), rs in sorted(by.items()):
            eds = ",".join(sorted({r["edition"] for r in rs}))
            labs = ",".join(f"{lab}:{n}" for lab, n in sorted(collections.Counter(meshcode.EPSG_LABEL.get(r["epsg"], f"EPSG:{r['epsg']}") for r in rs).items()))
            f.write(f"{m}\t{p}\t{len(rs)}\t{eds}\t{labs}\n")
