# AGENTS.md for /citygml-converter

Guidance for AI coding agents working in `/citygml-converter`. Read this before
touching anything here.

> `CLAUDE.md` in this directory is a symlink to this file. Edit `AGENTS.md`.

## What this is

A standalone CityGML 2.0 → 3.0 converter for PLATEAU city models, made of a
Rust core, a `citygml-convert` CLI, and (later) a Tauri desktop UI. Read
`README.md` first, since it covers the layout and the conversion pipeline.

## It is independent of the monorepo

`/citygml-converter` shares no code, no toolchain and no build with the rest
of PLATEAU-VIEW. Its own Cargo workspace, its own `rust-toolchain.toml`. Do not
add a dependency on `/server`, `/tile` or any JS workspace, and do not add
`citygml-converter/ui` to the root `package.json` workspaces.

It must work on Windows, macOS and Linux. No `std::os::unix`, no shelling out to
platform tools, no path separator assumptions. Normalise to `/` when comparing
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

* **A namespace bump, an element rename, an element to drop, or a value the
  converter has to invent** → a file under `profiles/` instead of Rust.
  This is a table, so keep it a table. *Which* file follows from what the rule
  varies with, and getting that wrong is how profiles drift apart:
  * true of CityGML 2.0 → 3.0 whatever i-UR the input carries →
    `citygml-2.0-to-3.0.toml`
  * true of producing i-UR 4.0 whatever version it came from →
    `iur-4.0-target.toml`, which is also where the code lists live
  * specific to one source i-UR minor → `iur-<version>-to-4.0.toml`

  The first two are *fragments* rather than profiles. Each profile names them
  in `base` and they fold in ahead of its own rules. Prefer a fragment. A rule
  that is not version-specific, written into one profile, is a rule the other
  two silently do without, which is the failure the split exists to prevent. The
  halves are disjoint on purpose, so a key declared on both sides is refused at
  load instead of resolved by precedence. There is no reading of "the fragment
  and the profile disagree" that is not a mistake in one of them.

  A rename that holds only under one parent takes `parent`, the parent's
  input name, and wins over a rule for the same `from` without one. It is for
  a name that means different things in different places, such as `uro:class`
  on `uro:OtherConstruction` and on `uro:FacilityTypeAttribute`. Write `parent`
  on the line after `to`, which is where the generator looks for it.
* **A mapping you cannot settle** → settle it, or turn it into a choice. An
  element either gets a rule, or becomes a policy table with a documented
  default the way `[lod4]` did, which is what to do when the answer turns on the
  data owner's intent rather than on the schemas. There is deliberately no
  flag-and-emit escape hatch. Emitting output you know is wrong and noting it
  afterwards is the worst outcome here, since half-converted output *looks*
  converted.
* **A child order** → nowhere. `core/src/schema.rs` reads every CityGML 3.0
  and i-UR 4.0 element's order off the schemas vendored in `fixtures/schemas/`,
  so a wrong order is fixed by updating those schemas. The one exception is a
  published schema that derives a type from the wrong base. That goes in
  `[schema_override]` with the base it `replaces`, which keeps the override
  inert once the published schema is corrected, and a test fails until the
  stale row is removed. Vendored schemas are never edited.
* **An i-UR rule that can be read off the schemas** → do not write it. Run the
  generator (`cargo run -p plateau-converter-gen -- --source 3.1 --write
  profiles/iur-3.1-to-4.0.toml`), which rewrites the block between the
  `# BEGIN/END GENERATED` markers. Everything outside those markers is
  hand-written and wins, and the generator skips any `from` already ruled on.
* **A rewrite that changes a value or the shape of a subtree** →
  `core/src/construction.rs` for those every construction shares (installations
  with their `con:relationToConstruction`, construction dates, `lodNGeometry`
  by the geometry it holds), which names no thematic module and reads the
  properties it acts on from the profile's `[construction]` table,
  `core/src/lodmap.rs` for a feature type whose 2.0 LODs move to another
  3.0 LOD (`[lod_map]`, such as a bridge's old LOD3 to LOD2, the moved LOD
  winning over what the slot held, and a feature whose moved LOD carries no
  `lodType` code given the `[lod_types]` table's `moved` code),
  `core/src/lod_type.rs` for adding a `lodType` code to a feature (shared by
  `lodmap` and `lod4`), `core/src/dropped.rs` for what the passes that
  remove geometry record and for writing back the removed geometry a kept
  link names (every removing pass hands what it removes to `Dropped::add`),
  `core/src/geometry.rs` for which `core`
  property a GML geometry goes to (no module knowledge, shared by
  `construction` and `unf`), `core/src/bldg.rs` for building-specific ones,
  `core/src/cons.rs` for other constructions (installations moving into the
  structure attribute), `core/src/iur.rs` for i-UR ones
  (the ADE hook rewrite), `core/src/lod4.rs` for where LOD4 goes (decided per
  feature from its measurement code and the profile's `[lod4]` table, never a
  rename row, since a `lod4*` rule in a profile would hide it from that pass,
  with the modules it folds, the elements marking the interior model and the
  feature types whose LOD4 ignores the code (`[lod4.fixed]`) and those whose
  LOD4 exterior shell is dropped instead of folded (`[lod4.exterior]`) listed
  in that table too, and the plain `lodType` codes it rewrites first, plus the codes a
  folded feature without a `4.x` code gains and the interior codes a `4.x`
  code becomes, read from the `[lod_types]` table, and the
  `urc:InteriorDataQualityAttribute` a folded interior model's descriptors
  move into),
  `core/src/xal.rs` for addresses (the xAL 2.0 content of `core:xalAddress`
  becomes an xAL 3.0 Address, and addresses appear on several feature types, so
  it is not part of `bldg.rs`), `core/src/app.rs` for the appearance module (the
  texture-to-surface binding, which every thematic module shares, so it is not
  part of `bldg.rs`), `common.rs` for those that apply to every feature type
  (generic attributes, lifespan dates). `measuredHeight` → `con:height` belongs
  in `bldg.rs` because it invents a `con:Height` object, while
  `bldg:lod1Solid` → `core:lod1Solid` is a table row.
* **A new thematic module** (`frn`, `veg`, …) → a sibling of `bldg.rs` plus its
  own profile rules, the way `core/src/tran.rs` and the `[tran]` table are for
  the transportation module, `core/src/unf.rs` and the `[unf]` table are for
  underground utilities, `core/src/wtr.rs` and the `[wtr]` table are for
  water bodies and `core/src/urf.rs` and the `[urf]` table are for urban
  function features. Do not grow `bldg.rs` sideways. A module deriving from
  the construction module (`brid`, `tun`, `cons`) adds rows to
  `[construction]`, `[lod4]`, `[lod_map]` and `[lod_types]` before it adds
  Rust. Rows naming
  i-UR source classes (`uro:OtherConstruction`) go in `iur-4.0-target.toml`,
  since they hold for every source minor.
* **A rewrite that moves content between top-level members** → read ahead
  with `xml::Reader::scan` and keep only the small part that moves, the way
  `core/src/storey.rs` collects floor groups (a 2.0 `grp:CityObjectGroup`
  becomes a `bldg:Storey` nested in its building, which may come earlier or
  later in the file). Never buffer the members themselves.
* **A geometry construction** → its own module with no module knowledge, the
  way `core/src/extrude.rs` turns a multi-surface into a solid and `tran.rs`
  decides when to call it. Coordinates are copied as strings, never
  reformatted, except the one value the construction changes.

`urf.rs`, `storey.rs`, `common.rs`, `unf.rs`, `lodmap.rs`, `lod4.rs`,
`construction.rs`, `bldg.rs`, `cons.rs`, `tran.rs`, `wtr.rs` and `iur.rs` run **after**
the rename pass, so they speak CityGML **3.0** and i-UR
**4.0** names only. Writing a 2.0 namespace constant in there is a bug
(`storey`'s read-ahead sees 2.0 input and maps each name through the profile
before comparing). The order is
`urf` -> `storey` -> `common` -> `xal` -> `app` -> `unf` -> `lodmap` -> `lod4` ->
`construction` -> `bldg` -> `cons` -> `tran` -> `wtr` -> `iur`, and each step is placed so that a wrapper it introduces is never
mistaken for something the next step handles. `urf` decides whether a member
is dropped before any pass works on it. `storey` nests each storey in its
building before the remaining passes run, so every later pass converts the storey's
generic attributes, dates and i-UR attributes along with the building's, and
`lod4` sees the storey CityGML 3.0 requires. `common`'s `core:genericAttribute`
is not a building property, `unf` has removed an underground utility's LOD4 and its
descriptors before `lod4` would read them as describing nothing, the geometry
`urf` drops that kept geometry of the same member still links to is written
back in place of the link right after `urf`, and likewise right after `unf`,
`lodmap`, `lod4` and `construction` for theirs (`core/src/dropped.rs`), a
member `urf` drops is recorded whole, and the appearance links to what stays dropped
are pruned right after that, member by member and on later members of the
same document (a document with an appearance member is first read with only
these steps, so an appearance member ahead of the features already knows
what they drop), `lodmap` has moved a feature's old LOD3
out of the LOD3 slots before `lod4` folds its LOD4 shell into them, and runs
before `construction` so a module's `lodNGeometry` moves as it is, `lod4` has
retagged every `lod4*` before
`construction` looks at geometry so it can never emit an LOD4 slot, the
storey check runs right after `lod4` and only when it kept an interior model,
`cons` moves installations into a structure attribute while it is still in its
2.0 property, before `iur` turns that into the ADE hook, `tran` has
renumbered the LOD-indexed quality descriptors before `iur` supplies the one
i-UR requires, `wtr` moves a water body's LOD1 into a new boundary surface
only after `lodmap` has decided which boundary surfaces stay, and `iur`'s
`bldg:adeOfAbstractBuilding` would be a building property if it ran before
`bldg`. `xal` and `app` touch only address and appearance content
respectively, which no later pass reads, so their slots are free.

## Invariants worth not breaking

* **Names stay expanded.** `xml::Name` is `(namespace uri, local name)`. Never
  compare or construct `prefix:local` strings outside the profile loader. The
  whole point is that a namespace change cannot be a string substitution.
* **Unmapped content passes through unchanged.** `uro:` attributes, `codeSpace`
  paths and every coordinate string must survive untouched. The integration
  tests assert this, and if a change makes them fail, the change is wrong.
* **Output is deterministic.** Generated `gml:id`s are seeded from the enclosing
  feature's own id and counted per feature, so two runs produce identical bytes.
  Do not introduce a global counter, a timestamp or anything order-dependent,
  since conversion is parallel across files.
* **Peak memory is one feature, not one file.** `xml::Reader` hands out one
  subtree per top-level member on purpose. Do not build a whole-document tree.
* **Paths a profile names are written from the package root.** A schema
  location or a `codeSpace` the converter writes itself is
  `schemas/...` or `codelists/...` in the profile, and gains the `../` steps
  back to the root from where each document sits (`profile::locate`), since
  `udx/` nests to different depths per feature type. Never write a `../`
  into a profile.
* **Assumptions are reported, never hidden.** When a conversion has to invent a
  value, add a `Warnings` entry saying what and why. `report::Warnings`
  deduplicates with counts, so per-feature messages are fine.

## Mapping rules

Mapping rules are experimental, and this experimental phase continues at least
until the official PLATEAU spec for CityGML 3.0 is released.