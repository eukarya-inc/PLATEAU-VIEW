# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "tifffile", "imagecodecs", "japan-geoid==0.6.0", "pytest"]
# ///
"""Vertical-reference COGs: fetch -> build -> validate -> publish -> verify.

    uv run vcog.py list
    uv run vcog.py fetch    <product|all>
    uv run vcog.py build    <product|all> [--diagnostic-merged]
    uv run vcog.py validate <product|all> [--no-calc] [--catalogue] [--oracle --listing-dem5a F --listing-dem1a F]
    uv run vcog.py publish  <product|all> [--dry-run] [--sample]
    uv run vcog.py verify   <product|all> [--sample]
    uv run vcog.py test

Products are declared in products.toml. Work files go to --work (default
./work, git-ignored). Storage/URL options default to products.toml [defaults].
"""

from __future__ import annotations

import argparse
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import products as prod  # noqa: E402


def _abs_repo(path: str) -> str:
    return path if os.path.isabs(path) else os.path.join(prod.REPO_ROOT, path)


def cmd_list(args, defaults, plist):
    for p in plist:
        print(f"{p.id:32s} {p.kind:18s} {p.keys_dir()}/")


def cmd_fetch(args, defaults, plist):
    import fetch

    for p in plist:
        fetch.fetch(p, args.work)


def cmd_build(args, defaults, plist):
    import build

    for p in plist:
        build.build(p, args.work, diagnostic_merged=args.diagnostic_merged)


def cmd_validate(args, defaults, plist):
    failed = False
    for p in plist:
        rd = p.report_dir(args.work)
        os.makedirs(rd, exist_ok=True)
        if p.kind == "geoid":
            import validate_geoid

            rep = validate_geoid.validate(os.path.join(p.out_dir(args.work), "geoid.tif"), p.get("validate"), calc=not args.no_calc)
            ok = rep["random_max_abs_diff_m"] < 1e-4 and rep["coverage_disagreements_in_bbox"] == 0 and rep["nodes_nan_in_cog_only"] == 0
            for s in rep.get("gsi_calculator", []):
                ok = ok and (s["diff_m"] is None or abs(s["diff_m"]) <= 1e-4)
            _write(rd, "geoid.json", rep)
            print(f"[{p.id}] crate max |d| {rep['random_max_abs_diff_m']:.2e} m, nodes max |d| {rep['nodes_max_abs_diff_m']:.2e} m, "
                  f"calculator {[s['diff_m'] for s in rep.get('gsi_calculator', [])]} -> {'OK' if ok else 'FAIL'}")
        else:
            ok = True
            if p.get("selection"):
                import check_fallback

                rep = check_fallback.check(p, args.work, catalogue=args.catalogue)
                _write(rd, "mesh_list_check.json", rep)
                print(f"[{p.id}] mesh list vs parameter files: {rep['candidates']} candidates, "
                      f"{len(rep['extra_candidates'])} not on the list, list entries not derivable: {rep['list_missing_from_candidates']}")
            if args.oracle:
                import validate_oracle

                specs = (p.get("oracle") or {}).get("meshes") or []
                diag = os.path.join(args.work, "out", "diagnostic", p.id, "merged.tif")
                report = validate_oracle.run(
                    p.out_dir(args.work), specs, os.path.join(args.work, "oracle"),
                    {"DEM5A": args.listing_dem5a, "DEM1A": args.listing_dem1a},
                    _abs_repo(args.rclone_config or defaults["rclone_config"]),
                    diag if os.path.exists(diag) else None,
                )
                _write(rd, "oracle.json", report)
                for e in report:
                    r = e["residual_m"]["gsi_rule"]
                    good = r["frac_le_0.005"] >= 0.99
                    ok = ok and good
                    print(f"[{p.id}] oracle {e['mesh']} {e['type']}: {r['frac_le_0.005'] * 100:.3f}% <=5 mm, max {r['max_abs'] * 1000:.2f} mm -> {'OK' if good else 'FAIL'}")
        failed = failed or not ok
    if failed:
        raise SystemExit(1)


def _write(d, name, obj):
    with open(os.path.join(d, name), "w", encoding="utf-8") as f:
        json.dump(obj, f, ensure_ascii=False, indent=2)


def _opts(args, defaults):
    return {
        "remote": args.remote or defaults["remote"],
        "bucket": args.bucket or defaults["bucket"],
        "rclone_config": _abs_repo(args.rclone_config or defaults["rclone_config"]),
        "allowed_prefix": args.allowed_prefix or defaults["allowed_prefix"],
        "public_base": args.public_base or defaults["public_base"],
        "config_url": args.config_url or defaults["config_url"],
    }


def cmd_publish(args, defaults, plist):
    import publish

    o = _opts(args, defaults)
    storage = publish.RcloneStorage(o["rclone_config"], o["remote"], o["bucket"])
    http = publish.UrllibHttp()
    try:
        for p in plist:
            rep = publish.publish(
                p, p.out_dir(args.work), storage, http,
                allowed_prefix=o["allowed_prefix"], config_url=o["config_url"], public_base=o["public_base"],
                dry_run=args.dry_run, sample=args.sample,
            )
            _write(_mk(p.report_dir(args.work)), "publish-dry-run.json" if args.dry_run else "publish.json", rep)
    except publish.PublishError as e:
        print(f"publish: {e}", file=sys.stderr)
        raise SystemExit(e.code) from None


def cmd_verify(args, defaults, plist):
    import publish

    o = _opts(args, defaults)
    bad = 0
    for p in plist:
        res = publish.verify(p, p.out_dir(args.work), publish.UrllibHttp(), allowed_prefix=o["allowed_prefix"], public_base=o["public_base"], sample=args.sample)
        bad += sum(not r["ok"] for r in res)
    if bad:
        raise SystemExit(f"{bad} object(s) differ from the local build")


def cmd_test(args, defaults, plist):
    import pytest

    raise SystemExit(pytest.main([os.path.join(HERE, "test_sampler.py"), os.path.join(HERE, "test_publish.py"), "-q"]))


def _mk(d):
    os.makedirs(d, exist_ok=True)
    return d


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--products", default=prod.DEFAULT_FILE)
    ap.add_argument("--work", default=os.path.join(os.getcwd(), "work"))
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("list", "fetch", "build", "validate", "publish", "verify", "test"):
        s = sub.add_parser(name)
        if name not in ("list", "test"):
            s.add_argument("product", help="product id or 'all'")
        if name == "build":
            s.add_argument("--diagnostic-merged", action="store_true", help="also write the not-published single-grid approximation")
        if name == "validate":
            s.add_argument("--no-calc", action="store_true", help="skip GSI's geoid calculator spot check")
            s.add_argument("--catalogue", action="store_true", help="look up extra mesh-list candidates in GSI's DEM catalogue")
            s.add_argument("--oracle", action="store_true", help="height-correction: compare with GSI's JGD2024 DEMs (login, <= 6 meshes)")
            s.add_argument("--listing-dem5a")
            s.add_argument("--listing-dem1a")
            s.add_argument("--rclone-config")
        if name in ("publish", "verify"):
            s.add_argument("--sample", action="store_true", help="also compare /vsicurl samples with the local COGs")
            s.add_argument("--remote")
            s.add_argument("--bucket")
            s.add_argument("--rclone-config", help="default: products.toml, relative to the repo root (env RCLONE_CONFIG_PATH)", default=os.environ.get("RCLONE_CONFIG_PATH"))
            s.add_argument("--allowed-prefix")
            s.add_argument("--public-base")
            s.add_argument("--config-url")
        if name == "publish":
            s.add_argument("--dry-run", action="store_true", help="read-only: check keys and config.json, print the plan")
    args = ap.parse_args()
    defaults, plist = prod.load(args.products)
    if args.cmd in ("list", "test"):
        selected = plist
    else:
        selected = prod.select(plist, args.product)
    globals()[f"cmd_{args.cmd}"](args, defaults, selected)


if __name__ == "__main__":
    main()
