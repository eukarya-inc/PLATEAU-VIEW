"""Load stacks.toml and map between stacks, primary meshes and bucket keys.

Work directory layout (default ``./work``, git-ignored)::

    work/zips/<PRODUCT>/FG-GML-<mesh6>-<PRODUCT>-<date>.zip   GSI downloads (fetch)
    work/src/<product>/FG-GML-...tif                          per-grid GeoTIFFs (gml2tif / mirror)
    work/src/provenance.jsonl                                 one line per grid written by gml2tif
    work/listings/<product>.json                              `rclone lsjson` of the backup (list-backup)
    work/inventory/<product>.jsonl                            CRS scan of the backup (inventory)
    work/out/<key>  + work/out/<key>.manifest.json            built COGs, laid out like the bucket
    work/reports/                                             strict / reproduce reports
"""

from __future__ import annotations

import os
import re
import tomllib
from dataclasses import dataclass
from fractions import Fraction

HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
DEFAULT_FILE = os.path.join(HERE, "stacks.toml")

KEY_RE = re.compile(r"^(base/dem\d+)/(\d{4})(-fill)?\.tif$")


class StackError(ValueError):
    pass


@dataclass(frozen=True)
class Stack:
    id: str
    key_prefix: str
    products: tuple[str, ...]  # bottom -> top
    pixel: Fraction

    def rank(self, product: str) -> int:
        try:
            return self.products.index(product.upper())
        except ValueError:
            raise StackError(f"{product} is not a product of stack {self.id} {self.products}") from None

    def key(self, primary: str, role: str) -> str:
        if role not in ("main", "fill"):
            raise StackError(f"bad role {role!r}")
        return f"{self.key_prefix}/{primary}{'-fill' if role == 'fill' else ''}.tif"


@dataclass(frozen=True)
class Patch:
    name: str
    src: str
    per_subdirectory: bool = False


@dataclass(frozen=True)
class Config:
    defaults: dict
    stacks: tuple[Stack, ...]
    patches: tuple[Patch, ...]

    def stack(self, sid: str) -> Stack:
        for s in self.stacks:
            if s.id == sid:
                return s
        raise StackError(f"unknown stack {sid!r}; known: {[s.id for s in self.stacks]}")

    def stack_of_product(self, product: str) -> Stack:
        for s in self.stacks:
            if product.upper() in s.products:
                return s
        raise StackError(f"no stack holds product {product!r}")

    def parse_key(self, key: str) -> tuple[Stack, str, str]:
        """``base/dem10/4730-fill.tif`` -> (dem10 stack, "4730", "fill")."""
        m = KEY_RE.match(key)
        if not m:
            raise StackError(f"{key!r} is not a base DEM key (base/demNN/<mesh4>[-fill].tif)")
        for s in self.stacks:
            if s.key_prefix == m[1]:
                return s, m[2], "fill" if m[3] else "main"
        raise StackError(f"{key!r}: no stack with key prefix {m[1]!r}")


def load(path: str = DEFAULT_FILE) -> Config:
    with open(path, "rb") as f:
        raw = tomllib.load(f)
    stacks = []
    for s in raw.get("stack", []):
        num, den = (int(x) for x in s["pixel"].split("/"))
        stacks.append(Stack(s["id"], s["key_prefix"].rstrip("/"), tuple(p.upper() for p in s["products"]), Fraction(num, den)))
    ids = [s.id for s in stacks]
    if len(set(ids)) != len(ids):
        raise StackError(f"duplicate stack ids in {path}")
    patches = tuple(Patch(p["name"], p["src"], bool(p.get("per_subdirectory"))) for p in raw.get("patch", []))
    return Config(raw.get("defaults", {}), tuple(stacks), patches)


def abs_repo(path: str) -> str:
    return path if os.path.isabs(path) else os.path.join(REPO_ROOT, path)
