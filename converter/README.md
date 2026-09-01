# PLATEAU CityGML 2.0 → 3.0 Converter

A standalone converter that upgrades PLATEAU5 3D city models from CityGML 2.0 to
CityGML 3.0. It targets Windows, macOS and Linux.

> **Status:** Work in Progress

## Layout

```
converter/
├── Cargo.toml                     workspace root
├── profiles/                      the declarative half of the mapping
│   ├── citygml-2.0-to-3.0.toml      shared fragment, the CityGML half
│   ├── iur-4.0-target.toml          shared fragment, the i-UR 4.0 target
│   ├── iur-3.0-to-4.0.toml          one profile per source i-UR version,
│   ├── iur-3.1-to-4.0.toml            each folding in both fragments
│   └── iur-3.2-to-4.0.toml
├── core/                          library for intake, XML, profile, transforms
├── cli/                           the `plateau-convert` binary
└── ui/                            Tauri desktop app (not started)
```

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

1. **One package**, a directory (or a zip of one) holding `udx/`, `codelists/`,
   `schemas/`. A directory is used *where it lies*, with nothing copied. A zip
   is extracted, and a single wrapping folder inside it is stripped.
2. **Loose per-part zips**, one archive each for `udx`, `codelists`, `schemas`.
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
stays at one building rather than one file. Names are kept expanded, as
`(namespace uri, local name)`, never as `prefix:local` strings. Prefixes are
resolved away on the way in and reinvented from the profile on the way out. That
is what makes moving `citygml/2.0` → `citygml/3.0` a data change rather than a
string-substitution hazard.

Each member then goes through three passes, in this order:

1. **Rename** (`transform::rename`), the profile's bulk namespace bump plus its
   per-element rules. Everything downstream therefore speaks 3.0 names only.
2. **Restructure** (`common`, `xal`, `app`, `lod4`, `bldg`, `iur`), the rewrites
   a rename table cannot express, because they change a value or the shape of a
   subtree. `lod4` is where CityGML 2.0 LOD4 goes, since 3.0 stops at LOD3 and
   splits a building into an exterior and an interior model. LOD4 is the
   interior model, and its LOD follows the measurement method, so a surveyed
   interior becomes interior LOD2 and a BIM-derived one interior LOD3. That is
   read from the feature's `geometrySrcDescLod4` code through the profile's
   `[lod4]` table, with a configurable fallback when no code decides. The LOD4
   exterior shell folds into the exterior LOD3 slot only where that slot is
   empty. The same pass rewrites the building `lodType` quality codes, which the
   i-UR 4.0 list splits into `_exterior`/`_interior` variants. Every fold,
   fallback and drop is reported. `app` is the appearance module, which every
   thematic module shares. Its attachment properties move into `core` as profile
   rows, and the texture-to-surface binding is rebuilt into
   `app:TextureAssociation` objects with `target` as an element, and `ring`
   attributes split into parallel elements.
3. **Reorder** (`transform::reorder`), because 3.0 content models are
   `xs:sequence`, so a property that was merely renamed can still land in the
   wrong slot.

Then `gml:id`s are minted for geometries that lack them, seeded from the
enclosing feature's own id so a re-run reproduces the same bytes.

Alongside the converted `udx/`, the output gets the vendored i-UR 4.0 schemas
and the published i-UR 4.0 code lists, both compiled into the binary, because
the published files are revised in place and conversion must not depend on a
network. The published lists replace the input's copies of the same files.
Lists the input authors itself (the profile's `[codelists] local` patterns) and
lists with no published counterpart are copied unchanged and win any name
collision with a published file. An input list that defines codes its published
counterpart lacks was edited by the municipality, which some lists exist for, so
it is kept in place of the published copy and reported. The exception is the
profile's `[codelists] superseded` patterns, lists whose values the conversion
itself rewrites into the published codes. After everything is written, any
`codeSpace` that resolves to no file in the output's `codelists/` is reported
rather than left for a validator to find.

Non-GML files inside a converted feature type, texture images above all, are
referenced by the documents by relative path and are copied unchanged into the
mirrored tree, so appearances keep resolving.

## The mapping profile

The source version is never asked for. Every PLATEAU document declares the i-UR
namespaces it uses, so the converter reads them and picks the matching profile.
The **target** is the part the data cannot supply, since an input says which
version it is and never which one it should become, so that is what
`--target-iur` selects. There is one profile per source i-UR version, because
the versions do not declare the same elements. 3.0 and 3.1 differ by 143 element
declarations, and five names they share changed the type they hold. `inspect`
shows which profile was chosen.

A profile is split along the axes it varies on, so that a rule which is not
specific to one source version is written once instead of copied three times and
left to drift. Two **fragments** hold the parts that do not vary, and each
profile names them in `base`:

* **`citygml-2.0-to-3.0.toml`**, everything true of CityGML 2.0 → 3.0 whatever
  i-UR the input carries: the CityGML prefixes on both sides, the CityGML
  `[namespace_map]` rows and remote `[output.schema_locations]`, the per-element
  renames and drops (`[[element]]`), the child order each 3.0 type requires
  (`[[order_group]]`), the `con:Height` parts CityGML 2.0 does not record
  (`[height]`), and where LOD4 goes (`[lod4]`, holding the measurement-code
  attribute, which codes send interior LOD4 content to LOD2 or LOD3, and the
  fallback of `lod3`, `lod2` or `drop` when no code decides).
* **`iur-4.0-target.toml`**, everything true of producing i-UR 4.0 whatever
  version it came from: `[target]`, the i-UR prefixes and their relative
  `[output.schema_locations]`, and `[codelists]`, which records which input code
  lists are municipality-authored and survive as shipped, which file names the
  published 4.0 set renamed (rewritten in every `codeSpace`), and which codes it
  dropped (kept and reported). This is the file that gains a sibling when i-UR
  publishes another minor, rather than the whole shared half being forked.
* **`iur-<version>-to-4.0.toml`**, what is specific to one source minor:
  `[source]`, that version's `uro`/`urf`/`urg`/`urt` prefixes and their
  `[namespace_map]` rows, the handful of i-UR rules that are a judgement call,
  and the generated block the schemas decide.

Fragments fold in ahead of the profile's own rules, and the two sides are
disjoint. A key declared in both is refused at load, naming both files, rather
than resolved by precedence. Fragments are named, not pathed, so a profile stays
one file.

All of it is compiled into the binary, and a profile can be replaced wholesale
with `--profile`. Such a file may name the built-in fragments or, declaring no
`base`, stand entirely on its own. An explicit profile is still checked against
the input, since pointing the 3.0 profile at 3.1 data converts the CityGML half
and writes every `uro:` element unqualified. The mismatch is reported rather
than left to be discovered in the output.

## Development

```bash
cargo test                                 # unit + integration, incl. the real PLATEAU fixture
cargo fmt --all
cargo clippy --all-targets -- -D warnings  # warnings are errors
```

`core/tests/fixtures/plateau` is a real PLATEAU 2023 Shizuoka package (LOD0
roof edge and LOD1 solid, `uro:` 3.0 attributes). The
integration tests convert it and assert on the result, so a regression in the
mapping shows up as a failing test rather than a bad output file.
