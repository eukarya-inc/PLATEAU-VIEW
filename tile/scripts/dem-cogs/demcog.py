# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "rasterio", "pytest"]
# ///
"""DEM COGs of the terrain stack (base/dem10, base/dem5, base/dem1, patch/).

Sources (pick one):
    uv run demcog.py fetch   <stack> <mesh4,...> [--before 2025-04-01 | --as-listed] [--login-conf F]
    uv run demcog.py gml2tif [zip ...]             # default: every zip under work/zips
    uv run demcog.py list-backup <product,...|all> [--hash]
    uv run demcog.py mirror  <stack> <mesh4,...>    # backup s1_geotiff_raw -> work/src
Build:
    uv run demcog.py build   <stack> <mesh4,...|all> [--as-served | --main-crs 3623=6668,...]
    uv run demcog.py patch   <name|all> [--src DIR]
Check:
    uv run demcog.py labels  [--mesh 473067] [MANIFEST ...]
    uv run demcog.py inventory <product,...|all>     # datum label of every backup grid
    uv run demcog.py strict  <stack> <mesh4,...|all> [--src DIR]   # served stack vs sources, per pixel
    uv run demcog.py reproduce <key> [<key> ...] [--src DIR]       # rebuild served COGs, compare pixels
Publish (dry run unless --execute):
    uv run demcog.py publish <key> [<key> ...] [--execute]
    uv run demcog.py test

Work files go to --work (default ./work, git-ignored). The backup and the
bucket are reached with the repo's rclone config (--rclone-config or
$RCLONE_CONFIG_PATH, default rclone.r2.conf at the repo root).
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import stacks as st  # noqa: E402

SERVED_STATE = os.path.join(HERE, "served", "state.json")


def _products(cfg: st.Config, arg: str) -> list[str]:
    allp = [p for s in cfg.stacks for p in s.products]
    if arg == "all":
        return allp
    out = [p.strip().upper() for p in arg.split(",")]
    for p in out:
        cfg.stack_of_product(p)
    return out


def _meshes(arg: str, tree=None, stack=None) -> list[str]:
    if arg == "all":
        if tree is None:
            raise SystemExit("'all' needs a source tree")
        return tree.primaries(stack.products)
    out = [m.strip() for m in arg.split(",") if m.strip()]
    for m in out:
        if not (len(m) == 4 and m.isdigit()):
            raise SystemExit(f"{m!r} is not a primary mesh code (4 digits)")
    return out


def _tree(args, cfg, remote_ok: bool):
    import sources

    if args.src == "backup":
        if not remote_ok:
            raise SystemExit("this command needs local grids: run `mirror` first, or pass --src DIR")
        return sources.SourceTree(sources.use_backup(cfg.defaults, args.rclone_config), os.path.join(args.work, "listings"))
    return sources.SourceTree(args.src or os.path.join(args.work, "src"))


def _write(path: str, obj) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(obj, f, ensure_ascii=False, indent=1)


# ---------------------------------------------------------------------------
def cmd_list_backup(args, cfg):
    import sources

    sources.list_backup(cfg.defaults, args.rclone_config, _products(cfg, args.products), os.path.join(args.work, "listings"), hashes=args.hash)


def cmd_inventory(args, cfg):
    import inventory

    inventory.run(cfg, args, _products(cfg, args.products))


def cmd_mirror(args, cfg):
    import sources

    stack = cfg.stack(args.stack)
    sources.mirror(cfg.defaults, args.rclone_config, os.path.join(args.work, "listings"), stack.products, _meshes(args.meshes), os.path.join(args.work, "src"))


def cmd_fetch(args, cfg):
    import fetch

    stack = cfg.stack(args.stack)
    products = _products(cfg, args.products) if args.products else list(stack.products)
    fetch.fetch(products, _meshes(args.meshes), os.path.join(args.work, "zips"), before=args.before,
                listing_dir=os.path.join(args.work, "listings") if args.as_listed else None,
                login_conf=args.login_conf or os.environ.get("GSI_LOGIN_CONF"))


def cmd_gml2tif(args, cfg):
    import fetch

    zips = args.zips or sorted(glob.glob(os.path.join(args.work, "zips", "*", "*.zip")))
    if not zips:
        raise SystemExit("no zips")
    fetch.gml2tif(zips, args.src or os.path.join(args.work, "src"), os.path.join(args.work, "zips", "fetch.jsonl"))


def cmd_build(args, cfg):
    import build
    import plan

    stack = cfg.stack(args.stack)
    tree = _tree(args, cfg, remote_ok=False)
    pins = plan.parse_main_pins(args.main_crs)
    if args.as_served:
        served = json.load(open(SERVED_STATE))["main_crs"].get(stack.id, {})
        pins = {**{k: int(v) for k, v in served.items()}, **pins}
    failed = []
    for m in _meshes(args.meshes, tree, stack):
        try:
            build.build_primary(stack, m, tree, os.path.join(args.work, "out"), main_epsg=pins.get(m), force=args.force)
        except (build.BuildError, plan.PlanError) as e:
            print(f"  {stack.id}/{m}: FAILED: {e}", file=sys.stderr)
            failed.append(m)
    if failed:
        raise SystemExit(f"{len(failed)} primary mesh(es) failed: {failed}")


def cmd_patch(args, cfg):
    import patch
    import sources

    todo = [p for p in cfg.patches if args.name in ("all", p.name, p.name.rstrip("/"))]
    if not todo:
        raise SystemExit(f"unknown patch {args.name!r}; known: {[p.name for p in cfg.patches]}")
    if args.src:
        if len(todo) != 1 or todo[0].per_subdirectory:
            raise SystemExit("--src needs exactly one single-directory patch")
        patch.build_patch(todo[0].name, args.src, os.path.join(args.work, "out"), force=args.force)
        return
    for p in todo:
        remote = f"{cfg.defaults['patch_backup']}/{p.src}".rstrip("/")
        subdirs = [""]
        if p.per_subdirectory:
            ls = subprocess.run([*sources.rclone_base(args.rclone_config), "lsf", "--dirs-only", remote], capture_output=True, text=True, check=True)
            subdirs = [d.rstrip("/") for d in ls.stdout.splitlines() if d.strip()]
        for sd in subdirs:
            name = p.name + sd if p.per_subdirectory else p.name
            local = os.path.join(args.work, "patch-src", *name.split("/"))
            subprocess.run([*sources.rclone_base(args.rclone_config), "copy", "--transfers", "16", f"{remote}/{sd}".rstrip("/"), local], check=True)
            patch.build_patch(name, local, os.path.join(args.work, "out"), force=args.force)


def cmd_labels(args, cfg):
    """Datum label per secondary mesh: from build manifests, or from the served-state table."""
    import build

    if args.served:
        path = os.path.join(HERE, "served", "dem10-labels.tsv")
        with open(path, encoding="utf-8") as f:
            header = f.readline()
            print(header, end="")
            for line in f:
                if not args.mesh or line.startswith(args.mesh[:6]):
                    print(line, end="")
        return
    paths = args.manifests or sorted(
        glob.glob(os.path.join(args.work, "out", "base", "**", "*.manifest.json"), recursive=True)
        + glob.glob(os.path.join(args.work, "repro", "base", "**", "*.manifest.json"), recursive=True))
    rows = build.labels_from_manifests(paths)
    if args.mesh:
        rows = [r for r in rows if r["mesh6"].startswith(args.mesh[:6])]
    print("mesh6\tproduct\tlabels\teditions\tgrids\tkey")
    for r in rows:
        labs = ",".join(f"{k}:{v}" for k, v in sorted(r["labels"].items()))
        print(f"{r['mesh6']}\t{r['product']}\t{labs}\t{','.join(r['editions'])}\t{r['grids']}\t{r['key']}")


def cmd_strict(args, cfg):
    import verify

    stack = cfg.stack(args.stack)
    tree = _tree(args, cfg, remote_ok=True)
    meshes = _meshes(args.meshes, tree, stack)
    report = os.path.join(args.work, "reports", f"strict-{stack.id}{'-' + args.meshes if args.meshes != 'all' and len(args.meshes) < 40 else ''}.json")
    s = verify.strict(cfg, stack, tree, meshes, report, threads=args.threads, exclude=tuple(args.exclude or ()))
    print(json.dumps({k: v for k, v in s.items() if k != "failed"}, indent=1))
    if s["failed"]:
        print(f"FAILED meshes ({len(s['failed'])}): {json.dumps(s['failed'])}")
    print(f"report: {report}")
    if s["failed"]:
        raise SystemExit(1)


def cmd_reproduce(args, cfg):
    import sources
    import verify

    bad = 0
    for key in args.keys:
        if key.startswith("patch/"):
            r = verify.reproduce_patch(cfg, key, _patch_src(args, cfg, key), args.work)
            _report_reproduce(args, r)
            bad += not r.get("pixel_identical")
            continue
        stack, primary, _ = cfg.parse_key(key)
        if args.src in (None, "backup"):
            sources.mirror(cfg.defaults, args.rclone_config, os.path.join(args.work, "listings"), stack.products, [primary], os.path.join(args.work, "src"))
            tree = sources.SourceTree(os.path.join(args.work, "src"))
        else:
            tree = sources.SourceTree(args.src)
        r = verify.reproduce_key(cfg, key, tree, args.work)
        _report_reproduce(args, r)
        bad += not r.get("pixel_identical")
    if bad:
        raise SystemExit(f"{bad} COG(s) not pixel-identical (see work/reports/reproduce/)")


def _report_reproduce(args, r: dict) -> None:
    _write(os.path.join(args.work, "reports", "reproduce", r["key"].replace("/", "_") + ".json"), r)
    line = {k: r.get(k) for k in ("key", "byte_identical", "same_grid", "same_nodata", "same_overviews", "pixels", "different_px", "pixel_identical")}
    if "attribution" in r:
        line["attribution"] = r["attribution"]["totals"]
    print(json.dumps(line, ensure_ascii=False))


def _patch_src(args, cfg, key: str) -> str:
    """Local copy of the source directory of patch ``key`` (from --src or the backup)."""
    import sources

    name = key[len("patch/"):-len(".tif")]
    if args.src and args.src != "backup":
        return args.src
    for p in cfg.patches:
        if p.name == name or (p.per_subdirectory and name.startswith(p.name)):
            sub = name[len(p.name):] if p.per_subdirectory else ""
            remote = f"{cfg.defaults['patch_backup']}/{p.src.rstrip('/')}" + (f"/{sub}" if sub else "")
            local = os.path.join(args.work, "patch-src", *name.split("/"))
            subprocess.run([*sources.rclone_base(args.rclone_config), "copy", "--transfers", "16", remote, local], check=True)
            return local
    raise SystemExit(f"{key}: no [[patch]] entry in stacks.toml")


def cmd_publish(args, cfg):
    import publish

    d = cfg.defaults
    storage = publish.RcloneStorage(args.rclone_config, d["remote"], d["bucket"])
    try:
        rep = publish.publish(os.path.join(args.work, "out"), args.keys, storage, publish.UrllibHttp(),
                              public_base=d["public_base"], config_url=d["config_url"], sources_json_url=d["sources_json_url"],
                              tile_source=d["tile_source"], execute=args.execute, pickup_timeout=args.pickup_timeout)
    except publish.PublishError as e:
        print(f"publish: {e}", file=sys.stderr)
        raise SystemExit(e.code) from None
    _write(os.path.join(args.work, "reports", "publish.json" if args.execute else "publish-dry-run.json"), rep)


def cmd_test(args, cfg):
    import pytest

    raise SystemExit(pytest.main([os.path.join(HERE, f) for f in sorted(os.listdir(HERE)) if f.startswith("test_") and f.endswith(".py")] + ["-q"]))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--stacks", default=st.DEFAULT_FILE)
    ap.add_argument("--work", default=os.path.join(os.getcwd(), "work"))
    ap.add_argument("--rclone-config", default=os.environ.get("RCLONE_CONFIG_PATH"), help="default: [defaults].rclone_config relative to the repo root")
    sub = ap.add_subparsers(dest="cmd", required=True)

    s = sub.add_parser("list-backup", help="rclone lsjson of the backup per product")
    s.add_argument("products")
    s.add_argument("--hash", action="store_true", help="also record MD5s (slow on dem5a)")
    s = sub.add_parser("inventory", help="datum label of every backup grid")
    s.add_argument("products")
    s.add_argument("--threads", type=int, default=32)
    s = sub.add_parser("mirror", help="copy backup grids of primary meshes to work/src")
    s.add_argument("stack")
    s.add_argument("meshes")
    s = sub.add_parser("fetch", help="download GSI zips (login)")
    s.add_argument("stack")
    s.add_argument("meshes")
    s.add_argument("--products", help="default: every product of the stack")
    g = s.add_mutually_exclusive_group()
    g.add_argument("--before", help="newest edition created before YYYY-MM-DD (2025-04-01 = JGD2011 era)")
    g.add_argument("--as-listed", action="store_true", help="exactly the editions in work/listings (the backup)")
    s.add_argument("--login-conf", help="GSI_USER=/GSI_PASS= file (default $GSI_LOGIN_CONF)")
    s = sub.add_parser("gml2tif", help="FG-GML zips -> per-grid GeoTIFFs + provenance")
    s.add_argument("zips", nargs="*")
    s.add_argument("--src", help="output source tree (default work/src)")
    s = sub.add_parser("build", help="per-primary COGs + manifests into work/out")
    s.add_argument("stack")
    s.add_argument("meshes")
    s.add_argument("--src", help="local source tree (default work/src)")
    s.add_argument("--main-crs", help="pin the main COG's CRS: 3623=6668,...")
    s.add_argument("--as-served", action="store_true", help="use the main-CRS pins of the served stack (served/state.json)")
    s.add_argument("--force", action="store_true")
    s = sub.add_parser("patch", help="patch/<name>.tif from local-government DEM tiles")
    s.add_argument("name")
    s.add_argument("--src", help="local input directory (default: copy from the backup)")
    s.add_argument("--force", action="store_true")
    s = sub.add_parser("labels", help="datum label per secondary mesh from build manifests")
    s.add_argument("manifests", nargs="*")
    s.add_argument("--mesh", help="secondary (or primary) mesh code prefix")
    s.add_argument("--served", action="store_true", help="query served/dem10-labels.tsv (every DEM10 grid of the served stack)")
    s = sub.add_parser("strict", help="served stack vs source grids, per pixel")
    s.add_argument("stack")
    s.add_argument("meshes")
    s.add_argument("--src", default="backup", help="'backup' (default, /vsis3 + work/listings) or a local source tree")
    s.add_argument("--threads", type=int, default=6)
    s.add_argument("--exclude", action="append", metavar="KEY", help="leave a served COG out (e.g. base/dem10/4931-fill.tif)")
    s = sub.add_parser("reproduce", help="rebuild served COGs and compare pixels")
    s.add_argument("keys", nargs="+")
    s.add_argument("--src", help="local source tree (default: mirror the backup into work/src)")
    s = sub.add_parser("publish", help="guarded upload (dry run unless --execute)")
    s.add_argument("keys", nargs="+")
    s.add_argument("--execute", action="store_true", help="really upload (the COGs go live within minutes)")
    s.add_argument("--pickup-timeout", type=float, default=420)
    sub.add_parser("test")

    args = ap.parse_args()
    cfg = st.load(args.stacks)
    args.rclone_config = st.abs_repo(args.rclone_config or cfg.defaults["rclone_config"])
    globals()[f"cmd_{args.cmd.replace('-', '_')}"](args, cfg)


if __name__ == "__main__":
    main()
