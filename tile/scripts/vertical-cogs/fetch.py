"""``vcog.py fetch``: download a product's GSI source files and record provenance.

Writes the extracted files plus ``sources.json`` (URL, sha256 of the archive
and of the extracted file, header/version line, fetch time) to
``work/src/<product id>/``. Public files come straight from www.gsi.go.jp;
sources marked ``login = true`` go through the 基盤地図情報 download service
(``GSI_LOGIN_CONF``, see gsi.py).
"""

from __future__ import annotations

import datetime
import hashlib
import html
import io
import os
import re
import zipfile

import gsi
from products import Product
from vref import sha256_file, write_json


def now_utc() -> str:
    return datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")


def zip_time(info: zipfile.ZipInfo) -> str:
    return "%04d-%02d-%02dT%02d:%02d:%02d" % info.date_time  # noqa: UP031


def first_line(data: bytes, encoding: str) -> str:
    return data.split(b"\n", 1)[0].decode(encoding, "replace").strip()


def parse_mesh_list(page_html: str, anchor: str, page: str) -> dict:
    """Extract a secondary-mesh list that follows ``anchor`` on a GSI page.

    GSI writes it as 「2次メッシュ番号：473113、473121、…」 (mixed 、/，separators).
    """
    text = re.sub(r"\s+", " ", html.unescape(re.sub(r"<[^>]+>", " ", page_html)))
    i = text.find(anchor)
    if i < 0:
        raise SystemExit(f"{page}: paragraph {anchor!r} not found")
    j = text.find("2次メッシュ番号：", i)
    k = text.find("今回整備", j)
    if j < 0 or k < 0:
        raise SystemExit(f"{page}: mesh list not found after {anchor!r}")
    quote = text[j:k].strip()
    meshes = sorted({int(m) for m in re.findall(r"\b\d{6}\b", quote)})
    if not meshes:
        raise SystemExit(f"{page}: empty mesh list")
    return {"quote": quote, "meshes": meshes}


def _extract(blob: bytes, member: str, dst_dir: str) -> tuple[str, bytes, zipfile.ZipInfo, list[str]]:
    with zipfile.ZipFile(io.BytesIO(blob)) as z:
        info = z.getinfo(member)
        data = z.read(info)
        names = z.namelist()
    dst = os.path.join(dst_dir, os.path.basename(member))
    with open(dst, "wb") as f:
        f.write(data)
    return dst, data, info, names


def fetch_source(cl: gsi.Client, src: dict, dst_dir: str) -> dict:
    if src.get("login"):
        cat = src["catalogue"]
        files = cl.geoid_files(cat["version"], cat["type"])
        if len(files) != 1:
            raise SystemExit(f"expected one {cat} package in GSI's catalogue, got {files}")
        meta = files[0]
        blob = cl.download_file(meta["id"])  # in memory; the archive is never written
        url = gsi.KIBAN_API + f"download/file/{meta['id']} (login required)"
        extra = {"catalogue": {k: meta[k] for k in ("id", "file_name", "file_update_date", "file_size_kbyte")}}
    else:
        blob = cl.get(src["url"])
        url = src["url"]
        extra = {}
    dst, data, info, names = _extract(blob, src["member"], dst_dir)
    enc = "cp932" if dst.endswith(".par") else "ascii"
    return {
        "url": url,
        "page": src.get("page"),
        "note": src.get("note"),
        **extra,
        "fetched_at": now_utc(),
        "archive_sha256": hashlib.sha256(blob).hexdigest(),
        "archive_bytes": len(blob),
        "archive_members": names,
        "file": os.path.basename(dst),
        "file_sha256": sha256_file(dst),
        "file_bytes": len(data),
        "file_mtime_in_zip": zip_time(info),
        "header": first_line(data, enc),
    }


def fetch(p: Product, work: str, cl: gsi.Client | None = None) -> dict:
    cl = cl or gsi.Client()
    d = p.src_dir(work)
    os.makedirs(d, exist_ok=True)
    out: dict[str, dict] = {}
    if p.kind == "height-correction":
        for g in p.get("grid"):
            out[g["name"]] = fetch_source(cl, g["source"], d)
            print(f"{p.id}/{g['name']}: {out[g['name']]['file']} {out[g['name']]['file_sha256']}")
        sel = p.get("selection")
        if sel and sel["kind"] == "listed-meshes":
            page = cl.get(sel["page"])
            out["mesh_list"] = {
                "page": sel["page"],
                "entry": sel.get("entry"),
                "fetched_at": now_utc(),
                "page_sha256": hashlib.sha256(page).hexdigest(),
                **parse_mesh_list(page.decode("utf-8", "replace"), sel["anchor"], sel["page"]),
            }
            print(f"{p.id}/mesh_list: {out['mesh_list']['meshes']}")
    else:
        out["geoid"] = fetch_source(cl, p.get("source"), d)
        print(f"{p.id}: {out['geoid']['file']} {out['geoid']['file_sha256']}")
    write_json(os.path.join(d, "sources.json"), out)
    return out
