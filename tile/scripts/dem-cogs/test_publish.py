"""Publish guards against a fake bucket, a fake config.json and a fake sources.json.

No network, no rclone. ``FakeHttp`` serves config.json from the fake bucket the
way the Worker does (every .tif under base/ patch/ sea/, sorted like
``demSource``), so a successful upload is "picked up" exactly when it lands.

    uv run demcog.py test
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import sys
import urllib.parse

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import publish  # noqa: E402

BASE = "https://tiles.example/terrain"
CONFIG = BASE + "/config.json"
SOURCES = "https://tile.example/tiles/sources.json"
EXISTING = ["base/dem10/4730.tif", "base/dem10/4931.tif", "patch/noto/central_noto.tif"]


def dem_priority(key):
    if key.startswith("sea/"):
        return 0
    m = re.match(r"^base/dem(\d+)/", key)
    if m:
        return 100 - int(m[1])
    return 200 if key.startswith("patch/") else 150


class FakeStorage:
    def __init__(self, existing=(), fail_put=None, fail_delete=False):
        self.objects: dict[str, bytes] = {k: b"old" for k in existing}
        self.puts, self.deletes = [], []
        self.fail_put, self.fail_delete = fail_put, fail_delete

    def exists(self, key):
        return key in self.objects

    def put(self, local, key, content_type):
        assert key not in self.objects, "overwrite attempted"
        if key == self.fail_put:
            raise RuntimeError("simulated upload failure")
        self.objects[key] = open(local, "rb").read()
        self.puts.append(key)

    def delete(self, key):
        if self.fail_delete:
            raise RuntimeError("simulated delete failure")
        del self.objects[key]
        self.deletes.append(key)


class FakeHttp:
    """config.json = the Worker's view of the fake bucket; sources.json = the tile server's (optionally stale)."""

    def __init__(self, storage, extra_layer=None, tile_server_lag=0, corrupt=None):
        self.s, self.extra_layer, self.lag, self.corrupt = storage, extra_layer, tile_server_lag, corrupt
        self.sources_reads = 0

    def _layers(self):
        keys = sorted(k for k in self.s.objects if k.endswith(".tif") and k.split("/")[0] in ("base", "patch", "sea"))
        keys.sort(key=lambda k: (dem_priority(k), k))
        urls = [f"{BASE}/{'/'.join(urllib.parse.quote(p) for p in k.split('/'))}" for k in keys]
        return urls

    def get(self, url):
        if url.startswith(CONFIG):
            urls = self._layers() + ([self.extra_layer] if self.extra_layer and self.s.puts else [])
            return json.dumps({"version": f"terrain-{len(urls)}", "sources": {"dem": {"type": "dem", "layers": [{"type": "cog", "url": u} for u in urls]}}}).encode()
        if url.startswith(SOURCES):
            self.sources_reads += 1
            urls = self._layers() if self.sources_reads > self.lag else [u for u in self._layers() if not any(u.endswith(p) for p in self.s.puts)]
            return json.dumps({"sources": [{"name": "dem", "type": "dem", "layers": []},
                                           {"name": "plateau-terrain-experimental", "type": "dem", "layers": [{"layer_index": i, "url": u} for i, u in enumerate(urls)]}]}).encode()
        key = urllib.parse.unquote(url[len(BASE) + 1:])
        body = self.s.objects[key]
        return body + b"x" if self.corrupt == key else body

    def head(self, url):
        key = urllib.parse.unquote(url[len(BASE) + 1:])
        if key not in self.s.objects:
            return 404, {}
        return 200, {"content-length": str(len(self.s.objects[key]))}


class Clock:
    def __init__(self):
        self.t = 0.0

    def __call__(self):
        return self.t

    def sleep(self, s):
        self.t += s


def built(tmp_path, keys):
    out = tmp_path / "out"
    for k in keys:
        p = out.joinpath(*k.split("/"))
        p.parent.mkdir(parents=True, exist_ok=True)
        body = f"cog {k}".encode()
        p.write_bytes(body)
        (p.parent / (p.name + ".manifest.json")).write_text(json.dumps({"key": k, "bytes": len(body), "sha256": hashlib.sha256(body).hexdigest()}))
    return str(out)


def run(out, keys, storage, http, **kw):
    c = Clock()
    return publish.publish(out, keys, storage, http, public_base=BASE, config_url=CONFIG, sources_json_url=SOURCES,
                           tile_source="plateau-terrain-experimental", poll_interval=15, pickup_timeout=120,
                           sleep=c.sleep, clock=c, log=lambda *a: None, **kw)


KEYS = ["base/dem10/4730-fill.tif", "base/dem10/4931-fill.tif"]


# --- keys -------------------------------------------------------------------
def test_dem_prefixes_match_the_worker():
    src = open(os.path.join(HERE, "..", "..", "..", "cloudflare", "tiles", "src", "r2.ts"), encoding="utf-8").read()
    m = re.search(r'isDem\s*\?\s*\[([^\]]*)\]', src)
    assert m, "tileConfig() default DEM prefixes not found in r2.ts"
    assert tuple(re.findall(r'"([^"]+)"', m[1])) == publish.DEM_CONFIG_PREFIXES


@pytest.mark.parametrize("key", ["base/dem10/4730.tif", "base/dem10/4730-fill.tif", "base/dem5/5339.tif", "patch/noto/x.tif", "sea/japan.tif"])
def test_good_cog_keys(key):
    assert publish.check_key(key) == "cog"


@pytest.mark.parametrize("key", ["/base/dem10/4730.tif", "base/dem10/../4730.tif", "base//dem10/4730.tif", "vertical/x.tif",
                                 "base/dem10/473.tif", "base/dem10/4730.vrt", "patch/x.tif", "plateau-terrain-2024/1/2/3.terrain",
                                 "manifests/base/dem10/4730.tif", "manifests/vertical/x.tif.manifest.json"])
def test_bad_keys(key):
    with pytest.raises(publish.PublishError):
        publish.check_key(key)


def test_manifest_key():
    assert publish.check_key("manifests/base/dem10/4730-fill.tif.manifest.json") == "manifest"


# --- plan -------------------------------------------------------------------
def test_plan_pairs_cogs_with_manifests(tmp_path):
    items = publish.plan(built(tmp_path, KEYS), KEYS)
    assert [i.key for i in items] == ["base/dem10/4730-fill.tif", "manifests/base/dem10/4730-fill.tif.manifest.json",
                                      "base/dem10/4931-fill.tif", "manifests/base/dem10/4931-fill.tif.manifest.json"]


def test_plan_refuses_manifest_mismatch(tmp_path):
    out = built(tmp_path, KEYS[:1])
    p = os.path.join(out, "base", "dem10", "4730-fill.tif")
    open(p, "ab").write(b"changed")
    with pytest.raises(publish.PublishError, match="differ"):
        publish.plan(out, KEYS[:1])
    with pytest.raises(publish.PublishError, match="missing"):
        publish.plan(out, ["base/dem10/5000.tif"])


def test_plan_refuses_manifest_for_other_key(tmp_path):
    out = built(tmp_path, KEYS[:1])
    mp = os.path.join(out, "base", "dem10", "4730-fill.tif.manifest.json")
    m = json.load(open(mp))
    m["key"] = "base/dem10/4931-fill.tif"
    json.dump(m, open(mp, "w"))
    with pytest.raises(publish.PublishError, match="is for"):
        publish.plan(out, KEYS[:1])


# --- dry run / no overwrite ----------------------------------------------------
def test_dry_run_writes_nothing(tmp_path):
    s = FakeStorage(EXISTING)
    r = run(built(tmp_path, KEYS), KEYS, s, FakeHttp(s))
    assert r["dry_run"] and s.puts == [] and r["config_before"]["layers"] == 3


def test_never_overwrites(tmp_path):
    s = FakeStorage(EXISTING + ["base/dem10/4931-fill.tif"])
    for execute in (False, True):
        with pytest.raises(publish.PublishError, match="never overwritten") as e:
            run(built(tmp_path, KEYS), KEYS, s, FakeHttp(s), execute=execute)
        assert e.value.code == 2
    assert s.puts == []


def test_existing_manifest_also_blocks(tmp_path):
    s = FakeStorage(EXISTING + ["manifests/base/dem10/4730-fill.tif.manifest.json"])
    with pytest.raises(publish.PublishError, match="never overwritten"):
        run(built(tmp_path, KEYS), KEYS, s, FakeHttp(s), execute=True)


# --- execute ------------------------------------------------------------------
def test_execute_happy_path(tmp_path):
    s = FakeStorage(EXISTING)
    r = run(built(tmp_path, KEYS), KEYS, s, FakeHttp(s, tile_server_lag=3), execute=True)
    assert set(s.puts) == {i["key"] for i in r["uploaded"]} and s.deletes == []
    assert r["config_after"]["layers"] == r["config_before"]["layers"] + 2
    assert all(c["ok"] for c in r["verify"])


def test_rollback_when_config_changes_otherwise(tmp_path):
    s = FakeStorage(EXISTING)
    with pytest.raises(publish.PublishError, match="unexpected new layers") as e:
        run(built(tmp_path, KEYS), KEYS, s, FakeHttp(s, extra_layer=f"{BASE}/base/dem10/9999.tif"), execute=True)
    assert e.value.code == 3 and sorted(s.deletes) == sorted(s.puts) and set(s.objects) == set(EXISTING)


def test_rollback_when_tile_server_never_picks_up(tmp_path):
    s = FakeStorage(EXISTING)
    with pytest.raises(publish.PublishError, match="sources.json pickup") as e:
        run(built(tmp_path, KEYS), KEYS, s, FakeHttp(s, tile_server_lag=10_000), execute=True)
    assert e.value.code == 3 and set(s.objects) == set(EXISTING)


def test_rollback_on_corrupt_public_object(tmp_path):
    s = FakeStorage(EXISTING)
    with pytest.raises(publish.PublishError, match="public verification") as e:
        run(built(tmp_path, KEYS), KEYS, s, FakeHttp(s, corrupt="base/dem10/4931-fill.tif"), execute=True)
    assert e.value.code == 4 and set(s.objects) == set(EXISTING)


def test_rollback_on_upload_error_keeps_going(tmp_path):
    s = FakeStorage(EXISTING, fail_put="base/dem10/4931-fill.tif")
    with pytest.raises(publish.PublishError, match="unexpected error") as e:
        run(built(tmp_path, KEYS), KEYS, s, FakeHttp(s), execute=True)
    assert e.value.code == 5 and set(s.objects) == set(EXISTING) and len(s.deletes) == 2
    assert "NOT deleted" in str(e.value) and "base/dem10/4931-fill.tif" in str(e.value)


def test_failed_put_never_deletes_someone_elses_object(tmp_path):
    class Racing(FakeStorage):
        def put(self, local, key, content_type):
            if key == "base/dem10/4931-fill.tif":
                self.objects[key] = b"another writer"  # appeared after the preflight
                raise RuntimeError("immutable: destination exists")
            super().put(local, key, content_type)

    s = Racing(EXISTING)
    with pytest.raises(publish.PublishError, match="NOT deleted"):
        run(built(tmp_path, KEYS), KEYS, s, FakeHttp(s), execute=True)
    assert s.objects["base/dem10/4931-fill.tif"] == b"another writer"
    assert "base/dem10/4931-fill.tif" not in s.deletes


def test_incomplete_rollback_names_leftovers(tmp_path):
    s = FakeStorage(EXISTING, fail_delete=True)
    with pytest.raises(publish.PublishError, match="STILL IN THE BUCKET") as e:
        run(built(tmp_path, KEYS), KEYS, s, FakeHttp(s, extra_layer=f"{BASE}/base/dem10/9999.tif"), execute=True)
    assert e.value.code == 6 and "base/dem10/4730-fill.tif" in str(e.value)


# --- pure helpers ---------------------------------------------------------------
def test_config_problems_paint_order():
    before = [f"{BASE}/base/dem10/4730.tif"]
    good = [f"{BASE}/base/dem10/4730-fill.tif", f"{BASE}/base/dem10/4730.tif"]
    assert publish.config_problems(before, good, [good[0]]) == []
    bad = list(reversed(good))
    assert any("painted above" in p for p in publish.config_problems(before, bad, [good[0]]))
    assert any("disappeared" in p for p in publish.config_problems(before + [f"{BASE}/x.tif"], good, [good[0]]))


def test_sources_json_is_a_list_of_named_objects():
    class H:
        def get(self, url):
            return json.dumps({"sources": [{"name": "a", "layers": [{"url": "u%E3%81%82"}]}]}).encode()

    assert publish.tile_source_layers(H(), SOURCES, "a") == ["uあ"]
    assert publish.tile_source_layers(H(), SOURCES, "b") is None

    class D:
        def get(self, url):
            return json.dumps({"sources": {"a": {}}}).encode()

    with pytest.raises(publish.PublishError, match="not a list"):
        publish.tile_source_layers(D(), SOURCES, "a")
