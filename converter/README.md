# PLATEAU CityGML 2.0 → 3.0 Converter

A standalone converter that upgrades PLATEAU5 3D city models from CityGML 2.0 to
CityGML 3.0. It targets Windows, macOS and Linux.

> **Status:** Work in Progress

## Quick start

```bash
cargo build --release

# A package directory, a zip of one, or several per-part zips, all the same call.
plateau-convert convert /data/22100_shizuoka-shi_city_2023_citygml_1_op -o ./out
plateau-convert convert 22100_..._op.zip -o ./out
plateau-convert convert udx.zip codelists.zip schemas.zip -o ./out

# See how the inputs were understood without converting anything.
plateau-convert inspect udx.zip codelists.zip
```

Useful flags:

| Flag | Effect |
| --- | --- |
| `-t, --type bldg` | which `udx/` feature types to convert (repeatable, `all` for everything) |
| `--target-iur VER` | which i-UR version to produce. One choice today (`4.0`), and errors listing the rest once there are more |
| `--profile FILE` | use a mapping profile from disk, overriding detection and `--target-iur` |
| `--staging DIR` | reassemble multi-part input here instead of a temp dir, and keep it |
| `--no-gml-ids` | do not mint the `gml:id`s GML 3.2 requires |
| `--no-reorder` | leave children in input order instead of the 3.0 sequence |
| `--lod4-fallback lod3\|lod2\|drop` | where LOD4 goes when no measurement code decides it (profile default `lod3`) |
| `--indent tab\|two\|four\|none` | output indentation |
| `-j N` | worker threads |

## How input is resolved

PLATEAU data arrives in two shapes, and both end up as one directory tree:

1. **One package**: a directory (or a zip of one) holding `udx/`, `codelists/`,
   `schemas/`. A directory is used where it lies, with nothing copied. A zip
   is extracted, and a single wrapping folder inside it is stripped.
2. **Loose per-part zips**: one archive each for `udx`, `codelists`, `schemas`.
   These are extracted into a staging directory laid out like case 1, because
   nothing can resolve a `codeSpace` or an `xsi:schemaLocation` until the parts
   sit next to each other again.

A part is recognised from the archive's own directory entries, failing that from
its file name (`..._udx.zip`), failing that from the file types inside (`.gml` →
`udx`, `.xsd` → `schemas`, `.xml` → `codelists`). Zip entries that try to escape
the staging directory are refused.

## Development

```bash
cargo test
cargo fmt --all
cargo clippy --all-targets -- -D warnings
```

`core/tests/fixtures/plateau` is a real PLATEAU 2023 Shizuoka package (LOD0
roof edge and LOD1 solid, `uro:` 3.0 attributes). The
integration tests convert it and assert on the result, so a regression in the
mapping shows up as a failing test rather than a bad output file.
