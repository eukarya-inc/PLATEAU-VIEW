"""``demcog.py fetch``: download 基盤地図情報 DEM zips from GSI, choosing editions explicitly.

The catalogue (no login) is ``/kiban/app/api/dem/update`` -- every edition
GSI still distributes, past and latest, one record per secondary-mesh zip
(``FG-GML-<mesh6>-<PRODUCT>-<YYYYMMDD>.zip``; the date is the newest
作成年月日 of the grids inside). Downloads need a login; the client is
``../vertical-cogs/gsi.py`` (credentials from a ``GSI_USER=``/``GSI_PASS=``
file whose path is ``--login-conf`` or ``$GSI_LOGIN_CONF``; read by Python
only, never printed or stored).

Edition choice -- one of:

* ``--as-listed <listings dir>``: exactly the edition the backup holds for
  each (product, secondary mesh): the zip whose date equals the newest grid
  edition in ``work/listings/<product>.json``. This is how the served stack
  is reproduced from GSI instead of the backup.
* ``--before YYYY-MM-DD``: the newest edition created before that date. Use
  ``--before 2025-04-01`` for the JGD2011-era stack: GSI re-issued DEM1A/5A/
  5B/5C on 2025-07-31 as JGD2024 (``fguuid:jgd2024.bl``) with editions
  dated 2025-04 onwards; ``gml2tif`` refuses those grids.
* neither: the latest edition (which for DEM5/DEM1 is now JGD2024).

Every downloaded zip gets a line in ``work/zips/fetch.jsonl`` (catalogue id,
file name, dates, size, sha256) that ``gml2tif`` copies into the grid
provenance.
"""

from __future__ import annotations

import collections
import datetime
import hashlib
import json
import os
import re
import sys
import time
import urllib.parse

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "vertical-cogs"))

import gsi  # noqa: E402  (../vertical-cogs/gsi.py)

import meshcode  # noqa: E402

ZIP_RE = re.compile(r"^FG-GML-(\d{6})-(DEM\w+)-(\d{8})\.zip$", re.IGNORECASE)


class FetchError(RuntimeError):
    pass


def catalogue(cl, products: list[str], primary: str) -> list[dict]:
    q = urllib.parse.urlencode({"date_from": "2008-01", "date_to": "2030-12", "type_codes": ",".join(products), "mesh_codes": primary})
    return cl.get_json(gsi.KIBAN_API + "dem/update?" + q).get("results") or []


def zip_date(rec: dict) -> str:
    m = ZIP_RE.match(rec["file_name"])
    if not m:
        raise FetchError(f"unexpected catalogue file name {rec['file_name']!r}")
    d = m[3]
    return f"{d[:4]}-{d[4:6]}-{d[6:]}"


def listed_editions(listing_dir: str, product: str, primary: str) -> dict[str, str]:
    """{mesh6: newest grid edition} of one product/primary in a backup listing."""
    out: dict[str, str] = {}
    for e in json.load(open(os.path.join(listing_dir, f"{product.lower()}.json"))):
        if e.get("IsDir"):
            continue
        g = meshcode.parse_name(e["Name"])
        if g.primary == primary and g.product == product:
            out[g.secondary] = max(out.get(g.secondary, ""), g.edition)
    return out


def select(records: list[dict], *, before: str | None = None, as_listed: dict[tuple[str, str], str] | None = None) -> list[dict]:
    """One record per (product, mesh6) -- see the module docstring (pure; tested)."""
    by: dict[tuple[str, str], list[dict]] = collections.defaultdict(list)
    for r in records:
        if r.get("file_split_seq_no", 0) not in (0, None):
            raise FetchError(f"{r['file_name']}: split archives are not supported")
        by[(r["type_code"].upper(), r["place_code"])].append(r)
    out = []
    if as_listed is not None:
        missing = sorted(set(as_listed) - set(by))
        if missing:
            raise FetchError(f"GSI's catalogue has no edition at all for {missing}")
        for k, date in sorted(as_listed.items()):
            hit = [r for r in by[k] if zip_date(r) == date]
            if len(hit) != 1:
                raise FetchError(f"{k}: the listing needs the {date} edition; GSI offers {sorted(zip_date(r) for r in by[k])}")
            out.append(hit[0])
        return out
    for _, rs in sorted(by.items()):
        rs = [r for r in rs if before is None or zip_date(r) < before]
        if rs:
            out.append(max(rs, key=zip_date))
    return out


def fetch(products: list[str], primaries: list[str], zip_root: str, *, before: str | None = None,
          listing_dir: str | None = None, login_conf: str | None = None, cl=None, log=print) -> list[dict]:
    if login_conf:
        os.environ["GSI_LOGIN_CONF"] = login_conf  # gsi.Client reads the file itself
    cl = cl or gsi.Client()
    os.makedirs(zip_root, exist_ok=True)
    rows = []
    with open(os.path.join(zip_root, "fetch.jsonl"), "a") as jl:
        for primary in primaries:
            recs = catalogue(cl, products, primary)
            as_listed = None
            if listing_dir:
                as_listed = {(p, m): d for p in products for m, d in listed_editions(listing_dir, p, primary).items()}
            chosen = select(recs, before=before, as_listed=as_listed)
            log(f"{primary}: {len(recs)} catalogue records, {len(chosen)} zips chosen")
            for r in chosen:
                d = os.path.join(zip_root, r["type_code"].upper())
                os.makedirs(d, exist_ok=True)
                dst = os.path.join(d, r["file_name"])
                if os.path.exists(dst):
                    data = open(dst, "rb").read()
                else:
                    data = cl.download_file(r["id"])
                    with open(dst + ".part", "wb") as f:
                        f.write(data)
                    os.replace(dst + ".part", dst)
                    time.sleep(1.0)  # be gentle with the service
                row = {"zip": r["file_name"], "gsi_id": r["id"], "gsi_info": r.get("info_name"),
                       "gsi_file_update_date": r.get("file_update_date"), "product": r["type_code"].upper(),
                       "mesh6": r["place_code"], "zip_bytes": len(data), "zip_sha256": hashlib.sha256(data).hexdigest(),
                       "fetched_at": datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")}
                jl.write(json.dumps(row, ensure_ascii=False) + "\n")
                rows.append(row)
                log(f"  {r['file_name']}  {len(data):,} B")
    return rows


def gml2tif(zips: list[str], src_root: str, fetch_log: str | None = None, log=print) -> list[dict]:
    """Convert zips into per-grid GeoTIFFs under ``src_root`` and append provenance."""
    import fggml

    meta = {}
    if fetch_log and os.path.exists(fetch_log):
        for line in open(fetch_log):
            r = json.loads(line)
            meta[r["zip"]] = r
    os.makedirs(src_root, exist_ok=True)
    out = []
    with open(os.path.join(src_root, "provenance.jsonl"), "a") as jl:
        for z in zips:
            rows = fggml.zip_to_tifs(z, src_root)
            m = meta.get(os.path.basename(z), {})
            for r in rows:
                for k in ("gsi_id", "gsi_file_update_date", "gsi_info"):
                    if k in m:
                        r[k] = m[k]
                jl.write(json.dumps(r, ensure_ascii=False) + "\n")
            labels = collections.Counter(r["label"] for r in rows)
            log(f"{os.path.basename(z)}: {len(rows)} grids {dict(labels)}")
            out += rows
    return out
