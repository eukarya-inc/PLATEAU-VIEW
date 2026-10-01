"""Tests for the publish guards, against fake storage and a fake config.json.

No network, no rclone: ``FakeStorage`` is a dict, ``FakeHttp`` serves the
config snapshots in sequence and the "public" objects from the fake bucket.

    uv run vcog.py test
"""

from __future__ import annotations

import hashlib
import json
import os
import sys

import pytest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import publish  # noqa: E402
from products import Product  # noqa: E402

BASE = "https://tiles.example/terrain"
CONFIG = BASE + "/config.json"
DEM_LAYERS = [f"{BASE}/base/dem10/{m}.tif" for m in (3036, 3622)]


class FakeStorage:
    def __init__(self, existing=None, fail_on=None):
        self.objects: dict[str, bytes] = dict(existing or {})
        self.puts: list[str] = []
        self.deletes: list[str] = []
        self.fail_on = fail_on

    def exists(self, key):
        return key in self.objects

    def put(self, local, key, content_type):
        if key == self.fail_on:
            raise RuntimeError("simulated upload failure")
        assert key not in self.objects, "test storage: overwrite attempted"
        with open(local, "rb") as f:
            self.objects[key] = f.read()
        self.puts.append(key)

    def delete(self, key):
        del self.objects[key]
        self.deletes.append(key)


class FakeHttp:
    def __init__(self, storage, configs):
        self.storage = storage
        self.configs = list(configs)  # consumed in order
        self.config_reads = 0

    def get(self, url):
        if url.startswith(CONFIG):
            self.config_reads += 1
            layers = self.configs.pop(0) if len(self.configs) > 1 else self.configs[0]
            return json.dumps({"version": f"terrain-{len(layers)}", "sources": {"dem": {"type": "dem", "layers": [{"type": "cog", "url": u} for u in layers]}}}).encode()
        return self.storage.objects[url[len(BASE) + 1 :]]

    def head(self, url):
        key = url[len(BASE) + 1 :]
        if key not in self.storage.objects:
            return 404, {}
        return 200, {"content-length": str(len(self.storage.objects[key]))}


@pytest.fixture
def product_dir(tmp_path):
    d = tmp_path / "out"
    d.mkdir()
    files = {"a.tif": b"tiff-bytes", "list.json": b'{"meshes": [1]}'}
    for n, b in files.items():
        (d / n).write_bytes(b)
    manifest = {"files": {n: {"sha256": hashlib.sha256(b).hexdigest(), "bytes": len(b)} for n, b in files.items()}}
    (d / "manifest.json").write_text(json.dumps(manifest))
    p = Product({"id": "x", "kind": "geoid", "version": "v1", "key_prefix": "vertical/x"})
    return p, str(d)


def run(p, d, storage, http, **kw):
    return publish.publish(p, d, storage, http, allowed_prefix="vertical/", config_url=CONFIG, public_base=BASE, log=lambda *a: None, **kw)


def test_happy_path_uploads_and_verifies(product_dir):
    p, d = product_dir
    st = FakeStorage()
    rep = run(p, d, st, FakeHttp(st, [DEM_LAYERS]))
    assert sorted(st.puts) == ["vertical/x/v1/a.tif", "vertical/x/v1/list.json", "vertical/x/v1/manifest.json"]
    assert st.deletes == []
    assert all(c["ok"] for c in rep["verify"])


def test_refuses_to_overwrite_an_existing_key(product_dir):
    p, d = product_dir
    st = FakeStorage(existing={"vertical/x/v1/a.tif": b"old"})
    http = FakeHttp(st, [DEM_LAYERS])
    with pytest.raises(publish.PublishError, match="already exist"):
        run(p, d, st, http)
    assert st.puts == [] and st.deletes == []
    assert st.objects["vertical/x/v1/a.tif"] == b"old"


def test_dry_run_also_refuses_existing_keys_and_never_writes(product_dir):
    p, d = product_dir
    st = FakeStorage(existing={"vertical/x/v1/manifest.json": b"{}"})
    with pytest.raises(publish.PublishError, match="already exist"):
        run(p, d, st, FakeHttp(st, [DEM_LAYERS]), dry_run=True)
    st2 = FakeStorage()
    rep = run(p, d, st2, FakeHttp(st2, [DEM_LAYERS]), dry_run=True)
    assert rep["dry_run"] and st2.puts == [] and st2.deletes == []


@pytest.mark.parametrize("prefix", ["vertical/../base/x", "base/x", "patch/x", "sea/x", "tiles/x", "/vertical/x"])
def test_forbidden_or_foreign_prefix(product_dir, prefix):
    p, d = product_dir
    p.raw["key_prefix"] = prefix
    st = FakeStorage()
    with pytest.raises(publish.PublishError):
        run(p, d, st, FakeHttp(st, [DEM_LAYERS]))
    assert st.puts == []


@pytest.mark.parametrize("allowed", ["", "/", "base/", "patch", "sea/sub/"])
def test_allowed_prefix_may_not_overlap_dem_prefixes(allowed):
    with pytest.raises(publish.PublishError):
        publish.check_allowed_prefix(allowed)


def test_dem_prefixes_match_the_config_worker():
    # cloudflare/tiles/src/r2.ts, tileConfig(): the default DEM prefixes.
    here = os.path.dirname(os.path.abspath(__file__))
    src = open(os.path.join(here, "..", "..", "..", "cloudflare", "tiles", "src", "r2.ts"), encoding="utf-8").read()
    assert '["base/", "patch/", "sea/"]' in src
    assert publish.DEM_CONFIG_PREFIXES == ("base/", "patch/", "sea/")


def test_manifest_mismatch_is_refused(product_dir):
    p, d = product_dir
    with open(os.path.join(d, "a.tif"), "ab") as f:
        f.write(b"!")
    st = FakeStorage()
    with pytest.raises(publish.PublishError, match="differ from manifest"):
        run(p, d, st, FakeHttp(st, [DEM_LAYERS]))
    os.remove(os.path.join(d, "a.tif"))
    with pytest.raises(publish.PublishError, match="files in manifest"):
        run(p, d, st, FakeHttp(st, [DEM_LAYERS]))
    assert st.puts == []


def test_rollback_when_an_uploaded_key_appears_as_a_layer(product_dir):
    p, d = product_dir
    st = FakeStorage()
    after = DEM_LAYERS + [f"{BASE}/vertical/x/v1/a.tif"]
    with pytest.raises(publish.PublishError, match="rolled back 3") as e:
        run(p, d, st, FakeHttp(st, [DEM_LAYERS, after]))
    assert e.value.code == 3
    assert sorted(st.deletes) == sorted(st.puts) and st.objects == {}


def test_rollback_when_config_changes_at_all(product_dir):
    p, d = product_dir
    st = FakeStorage(existing={"base/dem10/9999.tif": b"someone else's"})
    with pytest.raises(publish.PublishError, match="changed"):
        run(p, d, st, FakeHttp(st, [DEM_LAYERS, DEM_LAYERS + [f"{BASE}/base/dem10/9999.tif"]]))
    # only what this run uploaded is removed
    assert st.objects == {"base/dem10/9999.tif": b"someone else's"}


def test_refuses_when_config_already_lists_the_allowed_prefix(product_dir):
    p, d = product_dir
    st = FakeStorage()
    with pytest.raises(publish.PublishError, match="already lists"):
        run(p, d, st, FakeHttp(st, [DEM_LAYERS + [f"{BASE}/vertical/old.tif"]]))
    assert st.puts == []


def test_partial_upload_failure_rolls_back(product_dir):
    p, d = product_dir
    st = FakeStorage(fail_on="vertical/x/v1/list.json")
    with pytest.raises(publish.PublishError, match="upload failed"):
        run(p, d, st, FakeHttp(st, [DEM_LAYERS]))
    assert st.objects == {} and st.deletes == ["vertical/x/v1/a.tif"]


def test_public_verification_failure_rolls_back(product_dir, monkeypatch):
    p, d = product_dir
    st = FakeStorage()
    http = FakeHttp(st, [DEM_LAYERS])
    real_get = http.get
    http.get = lambda url: b"tampered" if url.endswith("a.tif") else real_get(url)
    monkeypatch.setattr(publish.time, "sleep", lambda s: None)
    with pytest.raises(publish.PublishError, match="verification failed") as e:
        run(p, d, st, http)
    assert e.value.code == 4 and st.objects == {}


def test_config_decision_unit():
    snap = {"version": "v", "layers": 2, "keyset_sha256": "h", "urls": DEM_LAYERS}
    assert publish.config_decision(snap, dict(snap), ["vertical/x/v1/a.tif"], "vertical/") == []
    hit = dict(snap, urls=DEM_LAYERS + [f"{BASE}/vertical/x/v1/a.tif"])
    assert publish.config_decision(snap, hit, ["vertical/x/v1/a.tif"], "vertical/")


def test_rclone_exists_does_not_trust_lsjson_stat(monkeypatch, tmp_path):
    # rclone answers `lsjson --stat` on a missing S3 object with a synthetic
    # directory and exit 0; exists() must list the parent instead.
    conf = tmp_path / "r.conf"
    conf.write_text("")
    calls = []

    class R:
        def __init__(self, out, rc=0, err=""):
            self.stdout, self.returncode, self.stderr = out, rc, err

    def fake_run(cmd, capture_output, text):
        calls.append(cmd)
        assert "lsjson" not in cmd and cmd[-1] == "r2:b/vertical/x/v1"
        return R("geoid.tif\nmanifest.json\nsub/\n")

    monkeypatch.setattr(publish.subprocess, "run", fake_run)
    s = publish.RcloneStorage(str(conf), "r2", "b")
    assert s.exists("vertical/x/v1/geoid.tif")
    assert s.exists("vertical/x/v1/sub")
    assert not s.exists("vertical/x/v1/other.tif")
    monkeypatch.setattr(publish.subprocess, "run", lambda *a, **k: R("", 1, "permission denied"))
    with pytest.raises(publish.PublishError):
        s.exists("vertical/x/v1/geoid.tif")
