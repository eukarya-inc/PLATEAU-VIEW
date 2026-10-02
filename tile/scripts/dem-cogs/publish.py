"""``demcog.py publish``: put built DEM COGs into the bucket -- where they go live.

**Anything uploaded under ``base/``, ``patch/`` or ``sea/`` is served within
minutes**: the terrain Worker lists those prefixes on every ``config.json``
request (``cloudflare/tiles/src/r2.ts`` ``tileConfig``, ``max-age=60``) and the
tile server revalidates that config about every 60 s (``CONFIG_TTL_SECS``)
into the ``plateau-terrain-experimental`` DEM source. There is no staging step.
So this module is deliberately conservative:

1. **Dry run by default.** Nothing is written without ``--execute``.
2. **Key guard.** COG keys must look like ``base/dem<NN>/<mesh4>[-fill].tif``,
   ``patch/<group>/<name>.tif`` or ``sea/<name>.tif`` (the prefixes the Worker
   enumerates, :data:`DEM_CONFIG_PREFIXES`, pinned to r2.ts by a test); each
   COG's manifest goes to ``manifests/<key>.manifest.json``, outside them.
   Absolute keys, ``..`` and empty segments are refused.
3. **The set is what the manifests say.** Each COG needs its
   ``<key>.manifest.json`` (from ``build``) naming that key, with matching
   size and sha256.
4. **Never overwrite.** Every key is checked by listing its parent directory
   (not ``rclone lsjson --stat``, which reports a missing S3 object as a
   directory with exit 0); any existing key aborts, also in a dry run.
   Uploads also use ``rclone copyto --immutable``. Replacing a served COG is
   not something this tool does.
5. **Public verification.** HEAD ``content-length`` and the sha256 of the body
   fetched from ``public_base`` must match the manifest.
6. **Pickup check.** ``config.json`` (cache-busted) is polled until it lists
   every uploaded COG and nothing else changed (layer set before + uploaded
   == after), and each ``-fill`` is painted under its main COG. Then the tile
   server's ``sources.json`` is polled until the ``tile_source`` source (a
   list of ``{"name", "layers": [...]}`` objects) lists them too.

Any failure after the first upload -- a failed check, a timeout, an
unexpected exception, Ctrl-C -- deletes exactly the objects this run uploaded
successfully. An object whose upload itself failed is not deleted (its
existence afterwards does not prove it is ours) and is named in the error.
If a delete fails, publish exits 6 and names every object left behind.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from typing import Protocol

# cloudflare/tiles/src/r2.ts tileConfig(): `isDem ? ["base/", "patch/", "sea/"]`.
DEM_CONFIG_PREFIXES = ("base/", "patch/", "sea/")
MANIFEST_PREFIX = "manifests/"
COG_KEY_RE = re.compile(r"^(base/dem\d+/\d{4}(-fill)?|patch/[^/]+/[^/]+|sea/[^/]+)\.tif$")

# Cloudflare in front of tiles.plateau.city answers 403 to Python's default UA.
USER_AGENT = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 dem-cogs"
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
# storage / http (tests inject fakes)
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

    def exists(self, key: str) -> bool:
        # Not `lsjson --stat <key>`: on bucket remotes rclone answers a missing
        # object with a synthetic directory entry and exit code 0. List the
        # parent (non-recursive); an object or "directory" of that name is taken.
        parent, _, name = key.rpartition("/")
        r = subprocess.run([*self.base, "lsf", "--max-depth", "1", f"{self.root}/{parent}"], capture_output=True, text=True)
        if r.returncode != 0 and "not found" not in r.stderr.lower():
            raise PublishError(f"cannot tell whether {key} exists: {r.stderr.strip()}")
        return any(line.rstrip("/") == name for line in r.stdout.splitlines())

    def put(self, local: str, key: str, content_type: str) -> None:
        subprocess.run(
            [*self.base, "copyto", "--immutable", "--s3-no-check-bucket", "--header-upload", f"Content-Type: {content_type}", local, f"{self.root}/{key}"],
            check=True,
        )

    def delete(self, key: str) -> None:
        subprocess.run([*self.base, "deletefile", f"{self.root}/{key}"], check=True)


class Http(Protocol):
    def get(self, url: str) -> bytes: ...
    def head(self, url: str) -> tuple[int, dict[str, str]]: ...


class UrllibHttp:
    def _req(self, url: str, method: str = "GET"):
        return urllib.request.Request(url, method=method, headers={"User-Agent": USER_AGENT, "Cache-Control": "no-cache"})

    def get(self, url: str) -> bytes:
        with urllib.request.urlopen(self._req(url), timeout=600) as r:
            return r.read()

    def head(self, url: str) -> tuple[int, dict[str, str]]:
        try:
            with urllib.request.urlopen(self._req(url, "HEAD"), timeout=60) as r:
                return r.status, {k.lower(): v for k, v in r.headers.items()}
        except urllib.error.HTTPError as e:
            return e.code, {}


def cache_busted(url: str) -> str:
    # tileConfig only reads `prefix` and `name`; any other parameter bypasses the 60 s CDN cache.
    return f"{url}{'&' if '?' in url else '?'}_demcog={time.time_ns()}"


# ---------------------------------------------------------------------------
# guards
# ---------------------------------------------------------------------------
def check_key(key: str) -> str:
    """'cog' or 'manifest'; raises for anything else."""
    parts = key.split("/")
    if key.startswith("/") or ".." in parts or "" in parts or "." in parts:
        raise PublishError(f"malformed key {key!r}")
    if key.startswith(MANIFEST_PREFIX):
        cog = key[len(MANIFEST_PREFIX):]
        if not cog.endswith(".manifest.json") or not COG_KEY_RE.match(cog[: -len(".manifest.json")]):
            raise PublishError(f"manifest key {key!r} must be {MANIFEST_PREFIX}<cog key>.manifest.json")
        return "manifest"
    if not any(key.startswith(p) for p in DEM_CONFIG_PREFIXES):
        raise PublishError(f"key {key!r} is outside the DEM prefixes {DEM_CONFIG_PREFIXES}")
    if not COG_KEY_RE.match(key):
        raise PublishError(f"key {key!r} does not look like a DEM COG key (base/demNN/<mesh4>[-fill].tif, patch/<group>/<name>.tif, sea/<name>.tif)")
    return "cog"


def sha256_of(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def plan(out_root: str, keys: list[str]) -> list[Item]:
    """COG keys -> [COG item, manifest item] each, checked against the manifests."""
    if not keys:
        raise PublishError("no keys given")
    items: list[Item] = []
    for key in keys:
        if check_key(key) != "cog":
            raise PublishError(f"{key!r}: pass COG keys; their manifests are added automatically")
        local = os.path.join(out_root, *key.split("/"))
        mpath = local + ".manifest.json"
        for p in (local, mpath):
            if not os.path.isfile(p):
                raise PublishError(f"{p} missing; run build first")
        with open(mpath, encoding="utf-8") as f:
            m = json.load(f)
        size, sha = os.path.getsize(local), sha256_of(local)
        if m.get("key") != key:
            raise PublishError(f"{mpath} is for {m.get('key')!r}, not {key!r}")
        if m.get("sha256") != sha or m.get("bytes") != size:
            raise PublishError(f"{local}: sha256/size differ from its manifest")
        mkey = f"{MANIFEST_PREFIX}{key}.manifest.json"
        check_key(mkey)
        items.append(Item(local, key, size, sha, CONTENT_TYPES[".tif"]))
        items.append(Item(mpath, mkey, os.path.getsize(mpath), sha256_of(mpath), CONTENT_TYPES[".json"]))
    if len({i.key for i in items}) != len(items):
        raise PublishError("duplicate keys")
    return items


# ---------------------------------------------------------------------------
# config.json / sources.json
# ---------------------------------------------------------------------------
def _decode(u: str) -> str:
    return urllib.parse.unquote(u)


def config_layers(http: Http, config_url: str) -> tuple[str | None, list[str]]:
    """(version, decoded layer URLs bottom -> top) of the DEM config (``sources`` is an object)."""
    doc = json.loads(http.get(cache_busted(config_url)))
    srcs = doc.get("sources", {})
    if not isinstance(srcs, dict):
        raise PublishError(f"{config_url}: 'sources' is not an object")
    return doc.get("version"), [_decode(l.get("url", "")) for s in srcs.values() for l in s.get("layers", [])]


def tile_source_layers(http: Http, sources_json_url: str, name: str) -> list[str] | None:
    """Decoded layer URLs of source ``name`` in the tile server's sources.json (``sources`` is a list)."""
    doc = json.loads(http.get(cache_busted(sources_json_url)))
    srcs = doc.get("sources", [])
    if not isinstance(srcs, list):
        raise PublishError(f"{sources_json_url}: 'sources' is not a list")
    for s in srcs:
        if s.get("name") == name:
            return [_decode(l.get("url", "")) for l in s.get("layers", [])]
    return None


def config_problems(before: list[str], after: list[str], uploaded_urls: list[str]) -> list[str]:
    """Why the after-config is not exactly before + uploaded (empty = fine)."""
    out = []
    missing = [u for u in uploaded_urls if u not in after]
    if missing:
        out.append(f"not yet listed: {missing}")
    expected = set(before) | set(uploaded_urls)
    extra, gone = sorted(set(after) - expected), sorted(expected - set(after) - set(missing))
    if extra:
        out.append(f"unexpected new layers: {extra}")
    if gone:
        out.append(f"layers disappeared: {gone}")
    for u in uploaded_urls:
        if u.endswith("-fill.tif") and u in after:
            main = u[: -len("-fill.tif")] + ".tif"
            if main in after and after.index(u) > after.index(main):
                out.append(f"{u} is painted above {main}")
    return out


def wait_until(check, timeout: float, interval: float, sleep=time.sleep, clock=time.monotonic):
    """Call ``check()`` (-> list of problems) until it returns [] or time runs out."""
    deadline = clock() + timeout
    while True:
        probs = check()
        if not probs or clock() >= deadline:
            return probs
        sleep(interval)


# ---------------------------------------------------------------------------
# public verification
# ---------------------------------------------------------------------------
def verify_items(items: list[Item], http: Http, public_base: str, retries: int = 5, sleep=time.sleep) -> list[dict]:
    out = []
    for it in items:
        url = f"{public_base.rstrip('/')}/{urllib.parse.quote(it.key)}"
        status, headers = 0, {}
        for attempt in range(retries):
            status, headers = http.head(url)
            if status == 200:
                break
            sleep(2 * (attempt + 1))
        cl = headers.get("content-length")
        body_sha = hashlib.sha256(http.get(url)).hexdigest() if status == 200 else None
        try:
            cl_ok = cl is not None and int(cl) == it.size
        except ValueError:
            cl_ok = False
        r = {"key": it.key, "status": status, "content_length": cl, "sha256_ok": body_sha == it.sha256}
        r["ok"] = status == 200 and cl_ok and r["sha256_ok"]
        out.append(r)
        print(json.dumps(r))
    return out


# ---------------------------------------------------------------------------
# publish
# ---------------------------------------------------------------------------
def publish(
    out_root: str, keys: list[str], storage: Storage, http: Http, *,
    public_base: str, config_url: str, sources_json_url: str, tile_source: str,
    execute: bool = False, pickup_timeout: float = 420, poll_interval: float = 15,
    sleep=time.sleep, clock=time.monotonic, log=print,
) -> dict:
    items = plan(out_root, keys)
    log(f"plan: {len(items)} objects")
    for it in items:
        log(f"  {it.key}  {it.size:,} B  sha256={it.sha256}")
    existing = [it.key for it in items if storage.exists(it.key)]
    if existing:
        raise PublishError(f"refusing: {len(existing)} key(s) already exist and are never overwritten: {existing}", 2)
    log("none of the keys exist")
    cog_urls = [_decode(f"{public_base.rstrip('/')}/{it.key}") for it in items if it.key.endswith(".tif")]
    version, before = config_layers(http, config_url)
    log(f"config.json before: version {version}, {len(before)} layers")
    already = [u for u in cog_urls if u in before]
    if already:
        raise PublishError(f"refusing: config.json already lists {already}", 2)
    if not execute:
        log(f"DRY RUN (pass --execute to upload): would upload {len(items)} objects with `rclone copyto --immutable`; "
            f"they would be listed by {config_url} within ~60 s and served by '{tile_source}' within a few minutes. "
            "Then: public sha256 check, config.json pickup (+ paint order of -fill), sources.json pickup; any failure rolls back.")
        return {"dry_run": True, "items": [it.__dict__ for it in items], "config_before": {"version": version, "layers": len(before)}}

    uploaded: list[str] = []
    uncertain: list[str] = []  # a put that failed: state unknown, left alone

    def rollback(why: str, code: int, cause: BaseException | None = None):
        log(f"ROLLBACK: {why}")
        left = []
        for k in reversed(uploaded):
            try:
                storage.delete(k)
                log(f"  deleted {k}")
            except BaseException as e:  # keep going
                left.append(k)
                log(f"  !! FAILED to delete {k}: {e!r}")
        note = (f" The upload of {uncertain} failed and was NOT deleted (it may be partial, ours, or another writer's): check it by hand."
                if uncertain else "")
        if left:
            raise PublishError(f"ROLLBACK INCOMPLETE after: {why}. STILL IN THE BUCKET (and possibly served), delete by hand: {left}.{note}", 6) from cause
        raise PublishError(f"rolled back {len(uploaded)} object(s): {why}.{note}", code) from cause

    class _Fail(Exception):
        def __init__(self, why: str, code: int):
            super().__init__(why)
            self.why, self.code = why, code

    try:
        for it in items:
            try:
                storage.put(it.local, it.key, it.content_type)
            except BaseException:
                # Existence after a failed put does not prove the object is ours
                # (another writer may have created the key since the preflight;
                # `--immutable` then fails). Never delete it: name it instead.
                uncertain.append(it.key)
                raise
            uploaded.append(it.key)
            log(f"  uploaded {it.key}")

        checks = verify_items(items, http, public_base, sleep=sleep)
        bad = [c["key"] for c in checks if not c["ok"]]
        if bad:
            raise _Fail(f"public verification failed for {bad}", 4)

        state: dict = {}

        def config_check():
            state["version"], state["after"] = config_layers(http, config_url)
            return config_problems(before, state["after"], cog_urls)

        probs = wait_until(config_check, pickup_timeout, poll_interval, sleep, clock)
        if probs:
            raise _Fail(f"config.json pickup: {probs}", 3)
        log(f"config.json after: version {state['version']}, {len(state['after'])} layers (+{len(cog_urls)})")

        def sources_check():
            layers = tile_source_layers(http, sources_json_url, tile_source)
            if layers is None:
                return [f"source {tile_source!r} not in sources.json"]
            return [f"{tile_source} does not list {u}" for u in cog_urls if u not in layers]

        probs = wait_until(sources_check, pickup_timeout, poll_interval, sleep, clock)
        if probs:
            raise _Fail(f"sources.json pickup: {probs}", 3)
        log(f"sources.json: '{tile_source}' lists all {len(cog_urls)} uploaded COG(s)")
    except _Fail as f:
        rollback(f.why, f.code)
    except BaseException as e:
        rollback(f"unexpected error after the upload started: {e!r}", 5, e)
    return {"uploaded": [it.__dict__ for it in items], "verify": checks,
            "config_before": {"version": version, "layers": len(before)},
            "config_after": {"version": state["version"], "layers": len(state["after"])}}
