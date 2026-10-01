"""``vcog.py publish`` / ``vcog.py verify``: place a built product in R2, safely.

The safety checks are the point of this module:

1. **Key guard.** Every key must sit under the allowed prefix (default
   ``vertical/``) and never under a prefix the DEM config generator lists
   (:data:`DEM_CONFIG_PREFIXES`) -- a correction grid listed there would be
   painted as terrain. ``..``/absolute keys are refused.
2. **Manifest guard.** The local directory must hold ``manifest.json`` and
   exactly the files it lists, with matching sha256 and size.
3. **No overwrite.** Each key is checked for existence first and publish
   refuses if any exists (also in ``--dry-run``). Uploads additionally use
   ``rclone copyto --immutable`` as a second line of defence.
4. **Config diff.** The dataset's ``config.json`` is fetched immediately
   before and after the upload. If an uploaded key shows up as a layer, or
   version / layer count / key-set hash changed at all, the objects uploaded
   in this run (and only those) are deleted and publish exits non-zero.
5. **Public verification.** Every object is fetched through the public URL:
   HEAD ``content-length`` and the sha256 of the body must match the
   manifest; ``--sample`` also reads each COG through GDAL ``/vsicurl/`` and
   requires the reference sampler to return bit-identical values to the
   local file. A failure rolls the upload back.

``--dry-run`` performs only reads (existence, config.json) and prints the
plan. No credentials live here: the rclone config path and remote name are
options (defaults from products.toml).
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Protocol

from products import Product

# Prefixes the terrain config Worker enumerates for DEM datasets by default:
# cloudflare/tiles/src/r2.ts, tileConfig(): `isDem ? ["base/", "patch/", "sea/"]`.
# Every COG under them becomes a DEM overlay layer. Keep in sync with r2.ts.
DEM_CONFIG_PREFIXES = ("base/", "patch/", "sea/")

# Cloudflare in front of tiles.plateau.city answers 403 to Python's default
# urllib User-Agent, so every request sends a browser-like one.
USER_AGENT = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 vertical-cogs"

CONTENT_TYPES = {".tif": "image/tiff", ".json": "application/json"}


class PublishError(Exception):
    def __init__(self, msg: str, code: int = 2):
        super().__init__(msg)
        self.code = code


@dataclass
class Item:
    local: str
    key: str
    size: int
    sha256: str
    content_type: str


# ---------------------------------------------------------------------------
# storage / http (real implementations; tests inject fakes)
# ---------------------------------------------------------------------------
class Storage(Protocol):
    def exists(self, key: str) -> bool: ...
    def put(self, local: str, key: str, content_type: str) -> None: ...
    def delete(self, key: str) -> None: ...


class RcloneStorage:
    def __init__(self, config: str, remote: str, bucket: str):
        if not os.path.exists(config):
            raise PublishError(f"rclone config {config} not found")
        self.base = ["rclone", "--config", config]
        self.root = f"{remote}:{bucket}"

    def _path(self, key: str) -> str:
        return f"{self.root}/{key}"

    def exists(self, key: str) -> bool:
        # Not `lsjson --stat <key>`: on bucket remotes rclone answers a
        # missing object with a synthetic directory entry and exit code 0.
        # List the parent directory (non-recursive) and look for the name;
        # an object or a "directory" of that name both count as taken.
        parent, _, name = key.rpartition("/")
        r = subprocess.run([*self.base, "lsf", "--max-depth", "1", f"{self.root}/{parent}"], capture_output=True, text=True)
        if r.returncode != 0 and "not found" not in r.stderr.lower():
            raise PublishError(f"cannot tell whether {key} exists: {r.stderr.strip()}")
        return any(line.rstrip("/") == name for line in r.stdout.splitlines())

    def put(self, local: str, key: str, content_type: str) -> None:
        subprocess.run(
            [*self.base, "copyto", "--immutable", "--s3-no-check-bucket", "--header-upload", f"Content-Type: {content_type}", local, self._path(key)],
            check=True,
        )

    def delete(self, key: str) -> None:
        subprocess.run([*self.base, "deletefile", self._path(key)], check=True)


class Http(Protocol):
    def get(self, url: str) -> bytes: ...
    def head(self, url: str) -> tuple[int, dict[str, str]]: ...


class UrllibHttp:
    def _req(self, url: str, method: str = "GET"):
        return urllib.request.Request(url, method=method, headers={"User-Agent": USER_AGENT, "Cache-Control": "no-cache"})

    def get(self, url: str) -> bytes:
        with urllib.request.urlopen(self._req(url), timeout=120) as r:
            return r.read()

    def head(self, url: str) -> tuple[int, dict[str, str]]:
        try:
            with urllib.request.urlopen(self._req(url, "HEAD"), timeout=60) as r:
                return r.status, {k.lower(): v for k, v in r.headers.items()}
        except urllib.error.HTTPError as e:
            return e.code, {}


# ---------------------------------------------------------------------------
# guards
# ---------------------------------------------------------------------------
def check_allowed_prefix(allowed_prefix: str) -> str:
    ap = allowed_prefix.strip("/") + "/"
    if ap == "/":
        raise PublishError("allowed prefix must not be the bucket root")
    for d in DEM_CONFIG_PREFIXES:
        if ap.startswith(d) or d.startswith(ap):
            raise PublishError(f"allowed prefix {ap!r} overlaps DEM config prefix {d!r}")
    return ap


def check_key(key: str, allowed_prefix: str) -> None:
    ap = check_allowed_prefix(allowed_prefix)
    parts = key.split("/")
    if key.startswith("/") or ".." in parts or "" in parts or "." in parts:
        raise PublishError(f"malformed key {key!r}")
    for d in DEM_CONFIG_PREFIXES:
        if key.startswith(d):
            raise PublishError(f"key {key!r} is under DEM config prefix {d!r}; it would be served as terrain")
    if not key.startswith(ap):
        raise PublishError(f"key {key!r} is outside the allowed prefix {ap!r}")


def sha256_of(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def plan(local_dir: str, keys_dir: str, allowed_prefix: str) -> list[Item]:
    """Files of ``local_dir`` -> keys ``<keys_dir>/<name>``, manifest-checked."""
    if not os.path.isdir(local_dir):
        raise PublishError(f"{local_dir} does not exist; run build first")
    names = sorted(os.listdir(local_dir))
    subdirs = [n for n in names if os.path.isdir(os.path.join(local_dir, n))]
    if subdirs:
        raise PublishError(f"{local_dir} contains directories {subdirs}; a product version is a flat set of files")
    if "manifest.json" not in names:
        raise PublishError(f"{local_dir}/manifest.json missing")
    with open(os.path.join(local_dir, "manifest.json"), encoding="utf-8") as f:
        listed = json.load(f).get("files", {})
    payload = [n for n in names if n != "manifest.json"]
    if set(payload) != set(listed):
        raise PublishError(f"files on disk {payload} != files in manifest {sorted(listed)}")
    items = []
    for n in names:
        path = os.path.join(local_dir, n)
        size, sha = os.path.getsize(path), sha256_of(path)
        if n != "manifest.json" and (listed[n]["sha256"] != sha or listed[n]["bytes"] != size):
            raise PublishError(f"{n}: sha256/size differ from manifest.json")
        key = f"{keys_dir.rstrip('/')}/{n}"
        check_key(key, allowed_prefix)
        ext = os.path.splitext(n)[1].lower()
        items.append(Item(path, key, size, sha, CONTENT_TYPES.get(ext, "application/octet-stream")))
    return items


# ---------------------------------------------------------------------------
# config.json snapshot and the rollback decision
# ---------------------------------------------------------------------------
def config_snapshot(http: Http, url: str) -> dict:
    # A throw-away query parameter bypasses the 60 s CDN cache; tileConfig
    # only reads `prefix` and `name`, so it does not change the result.
    sep = "&" if "?" in url else "?"
    doc = json.loads(http.get(f"{url}{sep}_vcog={time.time_ns()}"))
    layers = [l.get("url", "") for s in doc.get("sources", {}).values() for l in s.get("layers", [])]
    return {
        "version": doc.get("version"),
        "layers": len(layers),
        "keyset_sha256": hashlib.sha256("\n".join(sorted(layers)).encode()).hexdigest(),
        "urls": sorted(layers),
    }


def layers_hitting(snapshot: dict, keys: list[str], allowed_prefix: str) -> list[str]:
    ap = allowed_prefix.strip("/") + "/"
    return [u for u in snapshot["urls"] if f"/{ap}" in u or any(u.endswith("/" + k) for k in keys)]


def config_decision(before: dict, after: dict, uploaded: list[str], allowed_prefix: str) -> list[str]:
    """Reasons to roll back (empty list = keep the upload)."""
    reasons = []
    hits = layers_hitting(after, uploaded, allowed_prefix)
    if hits:
        reasons.append(f"uploaded/allowed-prefix keys appear as DEM layers: {hits}")
    for k in ("version", "layers", "keyset_sha256"):
        if before[k] != after[k]:
            reasons.append(f"config.json {k} changed: {before[k]} -> {after[k]}")
    return reasons


def _brief(s: dict) -> dict:
    return {k: s[k] for k in ("version", "layers", "keyset_sha256")}


# ---------------------------------------------------------------------------
# verification through the public URL
# ---------------------------------------------------------------------------
def verify_items(items: list[Item], http: Http, public_base: str, sample: bool = False, retries: int = 5) -> list[dict]:
    out = []
    for it in items:
        url = f"{public_base.rstrip('/')}/{it.key}"
        status, headers = 0, {}
        for attempt in range(retries):
            status, headers = http.head(url)
            if status == 200:
                break
            time.sleep(2 * (attempt + 1))
        cl = headers.get("content-length")
        body_sha = hashlib.sha256(http.get(url)).hexdigest() if status == 200 else None
        r = {"key": it.key, "url": url, "status": status, "content_length": cl, "sha256_ok": body_sha == it.sha256}
        r["ok"] = status == 200 and cl is not None and int(cl) == it.size and body_sha == it.sha256
        if sample and r["ok"] and it.key.endswith(".tif"):
            r["vsicurl_sample_exact"] = vsicurl_sample_exact(it.local, url)
            r["ok"] = r["ok"] and r["vsicurl_sample_exact"]
        out.append(r)
        print(json.dumps(r))
    return out


def vsicurl_sample_exact(local: str, url: str, n: int = 2000) -> bool:
    """Sample the public COG via GDAL /vsicurl/ and the local file; bit-equal?"""
    import numpy as np

    import sampler

    v = "/vsicurl/" + url
    with tempfile.TemporaryDirectory() as td:
        # GDAL_HTTP_USERAGENT: same Cloudflare 403 as above.
        env = dict(os.environ, GDAL_HTTP_USERAGENT=USER_AGENT)
        vrt = os.path.join(td, "g.vrt")
        # geotransform via VRT: `gdalinfo -json` prints it with only 15 digits
        subprocess.run(["gdal_translate", "-q", "-of", "VRT", v, vrt], check=True, env=env)
        with open(vrt) as f:
            x = f.read()
        gt = [float(s) for s in re.search(r"<GeoTransform>([^<]+)</GeoTransform>", x).group(1).split(",")]
        w = int(re.search(r'rasterXSize="(\d+)"', x).group(1))
        h = int(re.search(r'rasterYSize="(\d+)"', x).group(1))
        raw = os.path.join(td, "a.bin")
        subprocess.run(["gdal_translate", "-q", "-of", "ENVI", "-ot", "Float32", "-b", "1", v, raw], check=True, env=env)
        a = np.fromfile(raw, dtype="<f4").reshape(h, w)
    remote = sampler.from_array(a, gt[0], gt[3], gt[1], -gt[5])
    loc = sampler.open_cog(local)
    if (remote.x0, remote.y0, remote.dx, remote.dy, remote.data.shape) != (loc.x0, loc.y0, loc.dx, loc.dy, loc.data.shape):
        return False
    rows, cols = np.nonzero(~np.isnan(loc.data))
    rng = np.random.default_rng(1)
    pick = rng.integers(0, rows.size, n)
    # points around valid nodes, including cell interiors and exact nodes
    lat = loc.y0 - (rows[pick] + rng.uniform(0.0, 1.0, n)) * loc.dy
    lon = loc.x0 + (cols[pick] + rng.uniform(0.0, 1.0, n)) * loc.dx
    lat[:50] = loc.y0 - (rows[pick[:50]] + 0.5) * loc.dy
    lon[:50] = loc.x0 + (cols[pick[:50]] + 0.5) * loc.dx
    a1, b1 = sampler.sample_many(remote, lat, lon), sampler.sample_many(loc, lat, lon)
    return bool(np.array_equal(np.isnan(a1), np.isnan(b1)) and np.array_equal(a1[~np.isnan(a1)], b1[~np.isnan(b1)]))


# ---------------------------------------------------------------------------
# publish
# ---------------------------------------------------------------------------
def publish(
    p: Product,
    local_dir: str,
    storage: Storage,
    http: Http,
    *,
    allowed_prefix: str,
    config_url: str,
    public_base: str,
    dry_run: bool = False,
    sample: bool = False,
    log=print,
) -> dict:
    items = plan(local_dir, p.keys_dir(), allowed_prefix)
    log(f"[{p.id}] plan: {len(items)} objects under {p.keys_dir()}/")
    for it in items:
        log(f"  {it.key}  {it.size} B  sha256={it.sha256}  ({it.content_type})")

    existing = [it.key for it in items if storage.exists(it.key)]
    if existing:
        raise PublishError(f"[{p.id}] refusing: {len(existing)} key(s) already exist and are never overwritten: {existing}; bump the version", 2)
    log(f"[{p.id}] none of the keys exist")

    before = config_snapshot(http, config_url)
    log(f"[{p.id}] config.json before: {json.dumps(_brief(before))}")
    pre_hits = layers_hitting(before, [it.key for it in items], allowed_prefix)
    if pre_hits:
        raise PublishError(f"[{p.id}] refusing: config.json already lists layers under the allowed prefix: {pre_hits}", 2)

    if dry_run:
        log(f"[{p.id}] dry-run: would upload {len(items)} objects with `rclone copyto --immutable`, re-fetch {config_url}, "
            f"roll back on any change, then verify via {public_base}/<key> (HEAD content-length, sha256{', /vsicurl sampling' if sample else ''})")
        return {"product": p.id, "dry_run": True, "items": [it.__dict__ for it in items], "config_before": _brief(before)}

    uploaded: list[str] = []

    def rollback(why: str, code: int):
        log(f"[{p.id}] ROLLBACK: {why}")
        for k in reversed(uploaded):
            storage.delete(k)
            log(f"  deleted {k}")
        raise PublishError(f"[{p.id}] rolled back {len(uploaded)} object(s): {why}", code)

    try:
        for it in items:
            storage.put(it.local, it.key, it.content_type)
            uploaded.append(it.key)
            log(f"  uploaded {it.key}")
    except Exception as e:  # partial upload: undo what this run wrote
        rollback(f"upload failed: {e}", 3)

    after = config_snapshot(http, config_url)
    log(f"[{p.id}] config.json after: {json.dumps(_brief(after))}")
    reasons = config_decision(before, after, uploaded, allowed_prefix)
    if reasons:
        rollback("; ".join(reasons), 3)

    checks = verify_items(items, http, public_base, sample=sample)
    bad = [c["key"] for c in checks if not c["ok"]]
    if bad:
        rollback(f"public verification failed for {bad}", 4)
    return {"product": p.id, "uploaded": [it.__dict__ for it in items], "config_before": _brief(before), "config_after": _brief(after), "verify": checks}


def verify(p: Product, local_dir: str, http: Http, *, allowed_prefix: str, public_base: str, sample: bool = False) -> list[dict]:
    """Check already-published objects against the local build (read-only)."""
    items = plan(local_dir, p.keys_dir(), allowed_prefix)
    return verify_items(items, http, public_base, sample=sample, retries=1)
