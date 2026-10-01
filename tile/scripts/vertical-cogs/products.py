"""Load and check products.toml, and derive every path from it.

Layout of the work directory (default ``work/``, git-ignored)::

    work/src/<product id>/            downloaded source files + sources.json
    work/out/<key_prefix>/<version>/  built files, laid out exactly like the
                                      published keys (publish maps 1:1)
    work/reports/<product id>/        validation reports
"""

from __future__ import annotations

import os
import tomllib
from dataclasses import dataclass, field

HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
DEFAULT_FILE = os.path.join(HERE, "products.toml")

KINDS = ("height-correction", "geoid")
GEOID_FORMATS = ("isg", "gsigeo-asc")
SELECTION_KINDS = ("listed-meshes",)


@dataclass
class Product:
    raw: dict
    defaults: dict = field(default_factory=dict)

    @property
    def id(self) -> str:
        return self.raw["id"]

    @property
    def kind(self) -> str:
        return self.raw["kind"]

    @property
    def version(self) -> str:
        return self.raw["version"]

    @property
    def key_prefix(self) -> str:
        return self.raw["key_prefix"].rstrip("/")  # a leading "/" is kept so publish rejects it

    def keys_dir(self) -> str:
        """``<key_prefix>/<version>`` -- the published directory."""
        return f"{self.key_prefix}/{self.version}"

    def src_dir(self, work: str) -> str:
        return os.path.join(work, "src", self.id)

    def out_dir(self, work: str) -> str:
        return os.path.join(work, "out", *self.keys_dir().split("/"))

    def report_dir(self, work: str) -> str:
        return os.path.join(work, "reports", self.id)

    def get(self, key, default=None):
        return self.raw.get(key, default)


def load(path: str = DEFAULT_FILE) -> tuple[dict, list[Product]]:
    with open(path, "rb") as f:
        doc = tomllib.load(f)
    defaults = doc.get("defaults", {})
    products = [Product(p, defaults) for p in doc.get("product", [])]
    seen_ids, seen_dirs = set(), set()
    for p in products:
        for k in ("id", "kind", "version", "key_prefix"):
            if not p.get(k):
                raise ValueError(f"{path}: product {p.raw.get('id', '?')} lacks '{k}'")
        if p.kind not in KINDS:
            raise ValueError(f"{p.id}: unknown kind {p.kind!r} (expected one of {KINDS})")
        if p.id in seen_ids:
            raise ValueError(f"duplicate product id {p.id}")
        if p.keys_dir() in seen_dirs:
            raise ValueError(f"{p.id}: key directory {p.keys_dir()} used twice")
        seen_ids.add(p.id)
        seen_dirs.add(p.keys_dir())
        if p.kind == "height-correction":
            grids = p.get("grid") or []
            if not grids:
                raise ValueError(f"{p.id}: height-correction needs at least one [[product.grid]]")
            names = [g["name"] for g in grids]
            if len(set(names)) != len(names):
                raise ValueError(f"{p.id}: duplicate grid names {names}")
            sel = p.get("selection")
            if sel:
                if sel.get("kind") not in SELECTION_KINDS:
                    raise ValueError(f"{p.id}: unknown selection kind {sel.get('kind')!r}")
                for role in ("primary", "listed", "fallback"):
                    if sel.get(role) not in names:
                        raise ValueError(f"{p.id}: selection.{role} must name a grid ({names})")
            elif len(grids) != 1:
                raise ValueError(f"{p.id}: several grids need a [product.selection] rule")
        else:
            if p.get("format") not in GEOID_FORMATS:
                raise ValueError(f"{p.id}: geoid format must be one of {GEOID_FORMATS}")
            if not p.get("source"):
                raise ValueError(f"{p.id}: geoid needs [product.source]")
    return defaults, products


def select(products: list[Product], which: str) -> list[Product]:
    if which == "all":
        return products
    hit = [p for p in products if p.id == which]
    if not hit:
        raise SystemExit(f"unknown product {which!r}; known: {', '.join(p.id for p in products)}")
    return hit
