"""Where the per-grid GeoTIFFs come from: a local directory or the R2 backup.

A *source tree* has one sub-directory per product in lower case
(``dem10b/FG-GML-4730-67-dem10b-20161001.tif``) -- the layout of the backup's
``s1_geotiff_raw`` and of ``work/src`` (written by ``gml2tif`` or ``mirror``).

The backup is read through GDAL ``/vsis3/`` with the R2 keys of the repo's
rclone config, put into the environment of this process only (never printed).
Listing a backup directory is slow (dem5a has 287,681 files), so it is done
once with ``list-backup`` into ``work/listings/<product>.json`` and reused.
"""

from __future__ import annotations

import concurrent.futures as cf
import configparser
import json
import os
import re
import subprocess
from dataclasses import dataclass

import meshcode


@dataclass(frozen=True)
class SourceFile:
    grid: meshcode.GridName
    path: str  # local path or /vsis3/... path
    bytes: int | None = None
    md5: str | None = None

    @property
    def name(self) -> str:
        return self.grid.name


def rclone_base(rclone_config: str) -> list[str]:
    if not os.path.exists(rclone_config):
        raise SystemExit(f"rclone config {rclone_config} not found")
    return ["rclone", "--config", rclone_config]


def split_remote(remote_path: str) -> tuple[str, str]:
    """``r2:bucket/a/b`` -> (``r2``, ``bucket/a/b``)."""
    remote, sep, rest = remote_path.partition(":")
    if not sep:
        raise SystemExit(f"{remote_path!r} is not an rclone remote path")
    return remote, rest.strip("/")


def gdal_s3_env(rclone_config: str, remote: str) -> dict[str, str]:
    """GDAL /vsis3/ settings for an S3-type rclone remote (R2)."""
    c = configparser.ConfigParser()
    c.read(rclone_config)
    if remote not in c:
        raise SystemExit(f"remote {remote!r} not in {rclone_config}")
    r = c[remote]
    return {
        "AWS_ACCESS_KEY_ID": r["access_key_id"],
        "AWS_SECRET_ACCESS_KEY": r["secret_access_key"],
        "AWS_S3_ENDPOINT": re.sub(r"^https?://", "", r["endpoint"]).rstrip("/"),
        "AWS_VIRTUAL_HOSTING": "FALSE",
        "AWS_REGION": "auto",
        "GDAL_DISABLE_READDIR_ON_OPEN": "EMPTY_DIR",
        "CPL_VSIL_CURL_ALLOWED_EXTENSIONS": ".tif,.tiff,.xml",
        "GDAL_HTTP_MAX_RETRY": "5",
        "GDAL_HTTP_RETRY_DELAY": "2",
    }


def use_backup(defaults: dict, rclone_config: str) -> str:
    """Set the /vsis3/ environment and return the backup's /vsis3/ root."""
    remote, path = split_remote(defaults["backup_src"])
    os.environ.update(gdal_s3_env(rclone_config, remote))
    return "/vsis3/" + path


def list_backup(defaults: dict, rclone_config: str, products: list[str], out_dir: str, hashes: bool = False) -> None:
    """``rclone lsjson`` of each product directory. ``hashes`` adds the MD5
    (R2 ETag of single-part uploads); it is slow on the big directories."""
    os.makedirs(out_dir, exist_ok=True)
    for p in products:
        dst = os.path.join(out_dir, f"{p.lower()}.json")
        src = f"{defaults['backup_src']}/{p.lower()}"
        with open(dst + ".part", "w") as f:
            subprocess.run([*rclone_base(rclone_config), "lsjson", *(["--hash"] if hashes else []), "--no-mimetype", "--no-modtime", src], stdout=f, check=True)
        os.replace(dst + ".part", dst)
        print(f"{p}: {sum(1 for _ in json.load(open(dst)))} files -> {dst}")


class SourceTree:
    """Per-grid GeoTIFFs under ``root/<product lower>/``.

    ``root`` is a local directory (listed with ``os.listdir``) or a
    ``/vsi...`` path, which needs ``listing_dir`` with ``<product>.json``
    files from ``rclone lsjson`` (``list-backup``).
    """

    def __init__(self, root: str, listing_dir: str | None = None):
        self.root = root.rstrip("/")
        self.listing_dir = listing_dir
        self._cache: dict[str, list[SourceFile]] = {}

    @property
    def remote(self) -> bool:
        return self.root.startswith("/vsi")

    def files(self, product: str) -> list[SourceFile]:
        p = product.lower()
        if p in self._cache:
            return self._cache[p]
        out: list[SourceFile] = []
        if self.remote:
            if not self.listing_dir:
                raise SystemExit(f"{self.root} is remote: pass --listings (see list-backup)")
            lst = os.path.join(self.listing_dir, f"{p}.json")
            if not os.path.exists(lst):
                raise SystemExit(f"{lst} missing: run `demcog.py list-backup {product}` first")
            for e in json.load(open(lst)):
                if e.get("IsDir") or not e["Name"].lower().endswith(".tif"):
                    continue
                md5 = (e.get("Hashes") or {}).get("md5")
                out.append(SourceFile(meshcode.parse_name(e["Name"]), f"{self.root}/{p}/{e['Name']}", e.get("Size"), md5))
        else:
            d = os.path.join(self.root, p)
            if os.path.isdir(d):
                for n in sorted(os.listdir(d)):
                    if n.lower().endswith(".tif"):
                        out.append(SourceFile(meshcode.parse_name(n), os.path.join(d, n), os.path.getsize(os.path.join(d, n))))
        for f in out:
            if f.grid.product != product.upper():
                raise SystemExit(f"{f.path}: product {f.grid.product} found under {p}/")
        self._cache[p] = out
        return out

    def for_primary(self, products: tuple[str, ...] | list[str], primary: str) -> list[SourceFile]:
        return [f for p in products for f in self.files(p) if f.grid.primary == primary]

    def primaries(self, products) -> list[str]:
        return sorted({f.grid.primary for p in products for f in self.files(p)})


def read_epsg(path: str) -> int | None:
    import rasterio

    with rasterio.open(path) as d:
        return d.crs.to_epsg() if d.crs else None


def scan_epsg(files: list[SourceFile], cache_path: str | None = None, threads: int = 32) -> dict[str, int | None]:
    """EPSG of each file (opening only the header), cached as JSON lines by name."""
    known: dict[str, int | None] = {}
    if cache_path and os.path.exists(cache_path):
        for line in open(cache_path):
            r = json.loads(line)
            known[r["name"]] = r["epsg"]
    todo = [f for f in files if f.name not in known]
    if todo:
        os.makedirs(os.path.dirname(cache_path), exist_ok=True) if cache_path else None
        out = open(cache_path, "a") if cache_path else None
        try:
            with cf.ThreadPoolExecutor(threads) as ex:
                for i, (f, e) in enumerate(zip(todo, ex.map(lambda f: _epsg_or_error(f.path), todo), strict=True), 1):
                    known[f.name] = e
                    if out:
                        out.write(json.dumps({"name": f.name, "mesh": f.grid.mesh, "product": f.grid.product, "edition": f.grid.edition,
                                              "bytes": f.bytes, "md5": f.md5, "epsg": e}) + "\n")
                    if i % 5000 == 0:
                        print(f"  scanned {i}/{len(todo)}", flush=True)
        finally:
            if out:
                out.close()
    return {f.name: known[f.name] for f in files}


def _epsg_or_error(path: str) -> int | None:
    for _ in range(3):
        try:
            return read_epsg(path)
        except Exception:  # transient HTTP errors: retry, then record None (= refused later)
            continue
    return None


def mirror(defaults: dict, rclone_config: str, listing_dir: str, products, primaries: list[str], dst_root: str) -> int:
    """Copy the backup grids of ``primaries`` into ``dst_root/<product>/`` (rclone, resumable).

    Local copies are checked against the listing's sizes afterwards."""
    import tempfile

    tree = SourceTree("/vsis3/listing-only", listing_dir)  # only the listing is read, never the root
    n = 0
    for p in products:
        want = [f for f in tree.files(p) if f.grid.primary in set(primaries)]
        if not want:
            continue
        d = os.path.join(dst_root, p.lower())
        os.makedirs(d, exist_ok=True)
        with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False) as fl:
            fl.write("".join(f.name + "\n" for f in want))
        try:
            subprocess.run([*rclone_base(rclone_config), "copy", "--transfers", "32", "--checkers", "64",
                            "--files-from", fl.name, f"{defaults['backup_src']}/{p.lower()}", d], check=True)
        finally:
            os.unlink(fl.name)
        for f in want:
            local = os.path.join(d, f.name)
            if not os.path.exists(local) or (f.bytes is not None and os.path.getsize(local) != f.bytes):
                raise SystemExit(f"mirror: {local} missing or size differs from the listing")
        n += len(want)
        print(f"{p}: {len(want)} grids -> {d}")
    return n
