# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy"]
# ///
"""Download the GSI source files for the vertical-reference COGs.

    uv run fetch_sources.py [--out work/src]

Writes the extracted source files plus ``sources.json`` (URL, sha256 of the
archive and of each extracted file, header/version line) into ``--out``.
The PatchJGD(H) parameters and GSIGEO2011 are public; the JPGEO2024 ISG
package needs a 基盤地図情報 login (``GSI_LOGIN_CONF``, see gsi.py). Pass
``--skip-login`` to fetch only the public files.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import os
import zipfile

import gsi
from vref import sha256_file, write_json

GSI_COMMON = "https://www.gsi.go.jp/common/"

# Listed on https://www.gsi.go.jp/sokuchikijun/sokuchikijun41012.html#hyokorev2024
# (linked from https://www.gsi.go.jp/sokuchikijun/hyoko2024rev_param.html).
PUBLIC = [
    {
        "id": "hyokorevBM",
        "url": GSI_COMMON + "000268787.zip",
        "page": "https://www.gsi.go.jp/sokuchikijun/sokuchikijun41012.html#hyokorev2024",
        "member": "hyokorevBM_jgd2024_h.par",
        "note": "測地成果2024移行のための水準点標高補正パラメータ (published 2025-04-01)",
    },
    {
        "id": "hyokorevTR",
        "url": GSI_COMMON + "000268789.zip",
        "page": "https://www.gsi.go.jp/sokuchikijun/sokuchikijun41012.html#hyokorev2024",
        "member": "hyokorevTR_jgd2024_h.par",
        "note": "測地成果2024移行のための三角点標高補正パラメータ (published 2025-04-01)",
    },
    {
        "id": "gsigeo2011",
        "url": GSI_COMMON + "000275008.zip",
        "page": "https://www.gsi.go.jp/buturisokuchi/grageo_reference.html",
        "member": "gsigeo2011_ver2_2_asc/program/gsigeo2011_ver2_2.asc",
        "note": "日本のジオイド2011 (Ver.2.2)",
    },
]


def first_line(data: bytes, encoding: str) -> str:
    return data.split(b"\n", 1)[0].decode(encoding, "replace").strip()


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="work/src")
    ap.add_argument("--skip-login", action="store_true")
    args = ap.parse_args()
    os.makedirs(args.out, exist_ok=True)
    cl = gsi.Client()
    manifest: dict[str, dict] = {}

    for s in PUBLIC:
        blob = cl.get(s["url"])
        with zipfile.ZipFile(io.BytesIO(blob)) as z:
            info = z.getinfo(s["member"])
            data = z.read(info)
        dst = os.path.join(args.out, os.path.basename(s["member"]))
        with open(dst, "wb") as f:
            f.write(data)
        enc = "cp932" if dst.endswith(".par") else "ascii"
        manifest[s["id"]] = {
            "url": s["url"],
            "page": s["page"],
            "note": s["note"],
            "archive_sha256": hashlib.sha256(blob).hexdigest(),
            "archive_bytes": len(blob),
            "file": os.path.basename(dst),
            "file_sha256": sha256_file(dst),
            "file_bytes": len(data),
            "file_mtime_in_zip": "%04d-%02d-%02dT%02d:%02d:%02d" % info.date_time,
            "header": first_line(data, enc),
        }
        print(f"{s['id']}: {dst} {manifest[s['id']]['file_sha256']}")

    if not args.skip_login:
        # 基盤地図情報 ジオイド・モデル提供 page: https://service.gsi.go.jp/kiban/app/geoid/
        files = cl.geoid_files("2024", "ISG")
        if len(files) != 1:
            raise SystemExit(f"expected one ISG package for geoid 2024, got {files}")
        meta = files[0]
        blob = cl.download_file(meta["id"])
        with zipfile.ZipFile(io.BytesIO(blob)) as z:
            names = z.namelist()
            wanted = [n for n in names if n.lower().endswith(".isg")]
            extracted = {}
            for n in wanted:
                data = z.read(n)
                dst = os.path.join(args.out, os.path.basename(n))
                with open(dst, "wb") as f:
                    f.write(data)
                extracted[os.path.basename(n)] = {
                    "sha256": hashlib.sha256(data).hexdigest(),
                    "bytes": len(data),
                    "mtime_in_zip": "%04d-%02d-%02dT%02d:%02d:%02d" % z.getinfo(n).date_time,
                }
        manifest["jpgeo2024_isg"] = {
            "url": gsi.KIBAN_API + f"download/file/{meta['id']} (login required)",
            "page": "https://service.gsi.go.jp/kiban/app/geoid/",
            "catalogue": {k: meta[k] for k in ("id", "file_name", "file_update_date", "file_size_kbyte")},
            "archive_sha256": hashlib.sha256(blob).hexdigest(),
            "archive_bytes": len(blob),
            "archive_members": names,
            "files": extracted,
        }
        print(f"jpgeo2024 ISG: {sorted(extracted)}")

    write_json(os.path.join(args.out, "sources.json"), manifest)


if __name__ == "__main__":
    main()
