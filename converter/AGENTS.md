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
* **A mapping you cannot settle** → settle it, or turn it into a choice. An
  element either gets a rule, or — when the answer turns on the data owner's
  intent rather than on the schemas — becomes a policy table with a documented
  default, the way `[lod4]` did. There is deliberately no flag-and-emit escape
  hatch: emitting output you know is wrong and noting it afterwards is the worst
  outcome here — half-converted output that *looks* converted.
* **An i-UR rule that can be read off the schemas** → do not write it. Run the
  generator (`cargo run -p plateau-converter-gen -- --source 3.1 --write
  profiles/iur-3.1-to-4.0.toml`), which rewrites the block between the
  `# BEGIN/END GENERATED` markers. Everything outside those markers is
  hand-written and wins: the generator skips any `from` already ruled on.
* **A rewrite that changes a value or the shape of a subtree** →
  `core/src/bldg.rs` for building-specific ones, `core/src/iur.rs` for i-UR ones
  (the ADE hook rewrite), `core/src/lod4.rs` for where LOD4 goes (decided per
  feature from its measurement code and the profile's `[lod4]` table, never a
  rename row — a `lod4*` rule in a profile would hide it from that pass),
  `core/src/xal.rs` for addresses (the xAL 2.0 content of
  `core:xalAddress` becomes an xAL 3.0 Address; addresses appear on several
  feature types, so it is not part of `bldg.rs`), `core/src/app.rs` for the appearance module (the texture-to-surface binding;
  appearance is shared by every thematic module — `bldg`, `tran`, `frn` — so it
  is not part of `bldg.rs`), `common.rs` for those that apply to every feature
  type (generic attributes, lifespan dates). `measuredHeight` → `con:height`
  belongs in `bldg.rs` because it invents a `con:Height` object;
  `bldg:lod1Solid` → `core:lod1Solid` is a table row.
* **A new thematic module** (`tran`, `frn`, …) → a sibling of `bldg.rs` plus its
  own profile rules. Do not grow `bldg.rs` sideways.

`common.rs`, `lod4.rs`, `bldg.rs` and `iur.rs` run **after** the rename pass, so
they speak CityGML **3.0** and i-UR **4.0** names only. Writing a 2.0 namespace
constant in there is a bug. The order is `common` -> `xal` -> `app` -> `lod4` -> `bldg` -> `iur`,
and each step is placed so that a wrapper it introduces is never mistaken for
something the next step handles: `common`'s `core:genericAttribute` is not a
building property, `lod4` has retagged every `lod4*` before `bldg` looks at
geometry so `bldg` can never emit an LOD4 slot, and `iur`'s
`bldg:adeOfAbstractBuilding` would be a building property if it ran before
`bldg`. `xal` and
`app` touch only address and appearance content respectively, which no later
pass reads, so their slots are free.

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

`fixtures/schemas/iur/*/4.0/` are the i-UR 4.0 XSDs the converter writes into a
converted package; `fixtures/schemas/sources/` are the 3.x revisions the
generator reads. Both are
committed on purpose: i-UR publishes patch revisions **in place** under the same
minor-version URL and they are not compatible with one another, so fetching at
build or run time would make the output depend on when it ran. Replacing a set
means replacing all of it — the modules import one another by exact namespace.

`/fixtures/codelists` is vendored for the same reason: the published i-UR 4.0 code
lists, embedded by `core/build.rs` and written into every converted package in
place of the input's copies of the same files. Which input lists survive
instead (the municipality-authored ones), which file names the published set
moved, and which codes it dropped is the profile's `[codelists]` table — a
change there is a profile edit, not Rust. Never replace a municipality-authored
list with a published file of the same name: some published files under those
names are literal placeholder templates.

## Testing

`core/tests/fixtures/plateau` is a real PLATEAU 2023
Shizuoka package. Convert it in tests rather than hand-writing CityGML: a
regression in the mapping should surface as a failing assertion, not as a bad
output file someone notices later. Add a fixture only when the existing one
cannot exercise the path (LOD2 surfaces, `BuildingPart`, appearances).
