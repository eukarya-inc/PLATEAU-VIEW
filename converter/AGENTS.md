# AGENTS.md — /converter

Guidance for AI coding agents working in `/converter`. Read this before touching
anything here.

> `CLAUDE.md` in this directory is a symlink to this file. Edit `AGENTS.md`.

## What this is

A standalone CityGML 2.0 → 3.0 converter for PLATEAU city models: a Rust core, a
`plateau-convert` CLI, and (later) a Tauri desktop UI. Read `README.md` first —
it covers the layout and the conversion pipeline.

## It is independent of the monorepo

`/converter` shares no code, no toolchain and no build with the rest of
PLATEAU-VIEW. Its own Cargo workspace, its own `rust-toolchain.toml`. Do not add
a dependency on `/server`, `/tile` or any JS workspace, and do not add
`converter/ui` to the root `package.json` workspaces.

It must work on Windows, macOS and Linux. No `std::os::unix`, no shelling out to
platform tools, no path separator assumptions — normalise to `/` when comparing
relative paths, as `dataset.rs` does.

## Commands

```bash
cargo test                                 # unit + integration, incl. the real PLATEAU fixture
cargo fmt --all
cargo clippy --all-targets -- -D warnings  # warnings are errors
cargo run -p plateau-converter-cli -- convert <input>... -o out
cargo run -p plateau-converter-cli -- inspect <input>...
```

## Where a change belongs

The single most common mistake here is putting a mapping in the wrong layer.

* **A namespace bump, an element rename, an element to drop, a child order, or a
  value the converter has to invent** → `profiles/iur-<version>-to-4.0.toml`. No
  Rust. This is a table; keep it a table. There is one profile per source i-UR
  version and they are meant to stay in step, so a rule that is not
  version-specific belongs in all three.
* **A mapping you cannot settle** → the profile's `[[review]]` table. The element
  then passes through untouched and is named in the report. Do this instead of
  guessing: half-converted output that *looks* converted is the worst outcome
  here.
* **An i-UR rule that can be read off the schemas** → do not write it. Run the
  generator (`cargo run -p plateau-converter-gen -- --source 3.1 --write
  profiles/iur-3.1-to-4.0.toml`), which rewrites the block between the
  `# BEGIN/END GENERATED` markers. Everything outside those markers is
  hand-written and wins: the generator skips any `from` already ruled on.
* **A rewrite that changes a value or the shape of a subtree** →
  `core/src/bldg.rs` for building-specific ones, `core/src/iur.rs` for i-UR ones
  (the ADE hook rewrite),
  `common.rs` for those that apply to every feature type (generic attributes,
  lifespan dates). `measuredHeight` → `con:height` belongs in `bldg.rs` because
  it invents a `con:Height` object; `bldg:lod1Solid` → `core:lod1Solid` is a
  table row.
* **A new thematic module** (`tran`, `frn`, …) → a sibling of `bldg.rs` plus its
  own profile rules. Do not grow `bldg.rs` sideways.

`common.rs`, `bldg.rs` and `iur.rs` run **after** the rename pass, so they speak
CityGML **3.0** and i-UR **4.0** names only. Writing a 2.0 namespace constant in
there is a bug. The order is `common` -> `bldg` -> `iur`, and each step is placed
so that a wrapper it introduces is never mistaken for something the next step
handles: `common`'s `core:genericAttribute` is not a building property, and
`iur`'s `bldg:adeOfAbstractBuilding` would be if it ran before `bldg`.

## Invariants worth not breaking

* **Names stay expanded.** `xml::Name` is `(namespace uri, local name)`. Never
  compare or construct `prefix:local` strings outside the profile loader — the
  whole point is that a namespace change cannot be a string substitution.
* **Unmapped content passes through verbatim.** `uro:` attributes, `codeSpace`
  paths and every coordinate string must survive untouched. The integration
  tests assert this; if a change makes them fail, the change is wrong.
* **Output is deterministic.** Generated `gml:id`s are seeded from the enclosing
  feature's own id and counted per feature, so two runs produce identical bytes.
  Do not introduce a global counter, a timestamp or anything order-dependent —
  conversion is parallel across files.
* **Peak memory is one feature, not one file.** `xml::Reader` hands out one
  subtree per top-level member on purpose. Do not build a whole-document tree.
* **Assumptions are reported, never hidden.** When a conversion has to invent a
  value, add a `Warnings` entry saying what and why. `report::Warnings`
  deduplicates with counts, so per-feature messages are fine.

## Mapping rules

Mappring rules are experimental, and this experimental phase continues at least
until the official PLATEAU spec for CityGML3.0 is released.

## Vendored schemas

`schemas/iur/*/4.0/` are the i-UR 4.0 XSDs the converter writes into a converted
package; `schemas/sources/` are the 3.x revisions the generator reads. Both are
committed on purpose: i-UR publishes patch revisions **in place** under the same
minor-version URL and they are not compatible with one another, so fetching at
build or run time would make the output depend on when it ran. Replacing a set
means replacing all of it — the modules import one another by exact namespace.

## Testing

`core/tests/fixtures/plateau` is a real PLATEAU 2023
Shizuoka package. Convert it in tests rather than hand-writing CityGML: a
regression in the mapping should surface as a failing assertion, not as a bad
output file someone notices later. Add a fixture only when the existing one
cannot exercise the path (LOD2 surfaces, `BuildingPart`, appearances).
