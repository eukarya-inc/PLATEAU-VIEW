# PLATEAU CityGML 2.0 → 3.0 Converter

A standalone converter that upgrades PLATEAU5 3D city models from CityGML 2.0 to
CityGML 3.0. It targets Windows, macOS and Linux.

> **Status:** Work in Progress

## Layout

```
converter/
├── Cargo.toml                     workspace root
├── profiles/
│   ├── iur-3.0-to-4.0.toml        the declarative half of the mapping,
│   ├── iur-3.1-to-4.0.toml          one profile per source i-UR version
│   └── iur-3.2-to-4.0.toml
├── core/                          library: intake, XML, profile, transforms
├── cli/                           the `plateau-convert` binary
└── ui/                            Tauri desktop app (not started)
```

## Quick start

```bash
cargo build --release

# A package directory, a zip of one, or several per-part zips — all the same call.
plateau-convert convert /data/22100_shizuoka-shi_city_2023_citygml_1_op -o ./out
plateau-convert convert 22100_..._op.zip -o ./out
plateau-convert convert udx.zip codelists.zip schemas.zip -o ./out

# See how the inputs were understood without converting anything.
plateau-convert inspect udx.zip codelists.zip
```

Useful flags:

| Flag | Effect |
| --- | --- |
| `-t, --type bldg` | which `udx/` feature types to convert (repeatable; `all` for everything) |
| `--target-iur VER` | which i-UR version to produce. One choice today (`4.0`); errors listing the rest once there are more |
| `--profile FILE` | use a mapping profile from disk, overriding detection and `--target-iur` |
| `--staging DIR` | reassemble multi-part input here instead of a temp dir, and keep it |
| `--no-gml-ids` | do not mint the `gml:id`s GML 3.2 requires |
| `--no-reorder` | leave children in input order instead of the 3.0 sequence |
| `--indent tab\|two\|four\|none` | output indentation |
| `-j N` | worker threads |

## How input is resolved

PLATEAU data arrives in two shapes, and both end up as one directory tree:

1. **One package** — a directory (or a zip of one) holding `udx/`, `codelists/`,
   `schemas/`. A directory is used *where it lies*; nothing is copied. A zip is
   extracted, and a single wrapping folder inside it is stripped.
2. **Loose per-part zips** — one archive each for `udx`, `codelists`, `schemas`.
   These are extracted into a staging directory laid out like case 1, because
   nothing can resolve a `codeSpace` or an `xsi:schemaLocation` until the parts
   sit next to each other again.

A part is recognised from the archive's own directory entries, failing that from
its file name (`..._udx.zip`), failing that from the file types inside (`.gml` →
`udx`, `.xsd` → `schemas`, `.xml` → `codelists`). Zip entries that try to escape
the staging directory are refused.

## How conversion works

`Reader` streams a document and hands out **one materialised subtree per
top-level member**, so a feature can be restructured freely while peak memory
stays at one building rather than one file. Names are kept expanded —
`(namespace uri, local name)` — never as `prefix:local` strings; prefixes are
resolved away on the way in and reinvented from the profile on the way out. That
is what makes moving `citygml/2.0` → `citygml/3.0` a data change rather than a
string-substitution hazard.

Each member then goes through three passes, in this order:

1. **Rename** (`transform::rename`) — the profile's bulk namespace bump plus its
   per-element rules. Everything downstream therefore speaks 3.0 names only.
2. **Restructure** (`bldg::BuildingRewrite`) — the rewrites a rename table
   cannot express, because they change a value or the shape of a subtree.
3. **Reorder** (`transform::reorder`) — 3.0 content models are `xs:sequence`, so
   a property that was merely renamed can still land in the wrong slot.

Then `gml:id`s are minted for geometries that lack them, seeded from the
enclosing feature's own id so a re-run reproduces the same bytes.

## The mapping profile

The source version is never asked for: every PLATEAU document declares the i-UR
namespaces it uses, so the converter reads them and picks the matching profile.
The **target** is the part the data cannot supply — an input says which version
it is, never which one it should become — so that is what `--target-iur`
selects. There is one profile per source i-UR version, because the versions do
not declare the same elements: 3.0 and 3.1 differ by 143 element declarations, and
five names they share changed the type they hold. The converter reads the i-UR
namespaces the input declares and picks the matching profile; `inspect` shows
which one it chose. Each profile holds everything about the mapping that is a
table:

* `[source]` / `[target]` — the versions the profile accepts and produces, and
  what detection matches an input against.
* `[input.namespaces]` / `[output.namespaces]` — prefix tables for the two sides.
* `[namespace_map]` — the bulk namespace bump, including i-UR 3.x → 4.0.
* `[[element]]` — per-element renames and drops.
* `[[order_group]]` — the child order each 3.0 type requires.
* `[height]` — the `con:Height` parts CityGML 2.0 does not record.
* `[[review]]` — elements to leave alone and report, because their mapping needs
  a human decision.

All three are compiled into the binary, and any of them can be replaced
wholesale with `--profile`. An explicit profile is still checked against the
input: pointing the 3.0 profile at 3.1 data converts the CityGML half and writes
every `uro:` element unqualified, so the mismatch is reported rather than left to
be discovered in the output.

## Development

```bash
cargo test                                 # 71 tests, including the real PLATEAU fixture
cargo fmt --all
cargo clippy --all-targets -- -D warnings  # warnings are errors
```

`core/tests/fixtures/plateau` is a real PLATEAU 2023
Shizuoka package (LOD0 roof edge + LOD1 solid, `uro:` 3.0 attributes). The
integration tests convert it and assert on the result, so a regression in the
mapping shows up as a failing test rather than a bad output file.
