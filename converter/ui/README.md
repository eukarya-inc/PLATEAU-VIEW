# Converter UI (not started)

A Tauri desktop front end for `plateau-converter-core`, so the conversion can be
run by dropping a folder or a set of zips onto a window instead of assembling a
command line. Windows, macOS and Linux.

Nothing here yet. This file records the decisions already made so the scaffold
does not have to relitigate them.

## Intended shape

```
ui/
├── package.json          frontend (yarn, matching the rest of the monorepo)
├── src/                  frontend source
└── src-tauri/
    ├── Cargo.toml        depends on ../../core
    ├── tauri.conf.json
    └── src/lib.rs        commands wrapping Dataset + Converter
```

`src-tauri` becomes a member of the `converter/` Cargo workspace, and the slot
is already reserved in `converter/Cargo.toml`.

## What the core already exposes for it

The UI needs no new conversion logic. The pieces it will call are:

* `dataset::Dataset::open_with(&inputs, &Staging)` takes the same mixed list of
  directories and zips the CLI takes, so a drop target can pass paths straight
  through. `Staging::At(dir)` gives the UI a staging directory it controls.
* `Dataset::parts`, `feature_types` and `gml_files` are enough to render a
  summary of what was dropped before anything is converted, which is what
  `inspect` prints.
* `convert::Converter::convert_dataset` returns a `report::Report` with file and
  feature counts plus deduplicated warnings, ready to render as a result panel.
* `profile::Rules::from_toml` lets the UI ship a profile editor later without a
  rebuild.

## Open decisions

* Frontend framework, and whether to share anything with `/editor` or
  `/extension` (probably not, since this ships as a desktop binary rather than a
  web app).
* Progress reporting. `convert_dataset` currently returns only when finished, so
  a per-file callback or a channel will be needed for a progress bar.
* Cancellation. Same place, since the rayon fan-out in `convert_dataset` needs
  an abort flag.
* Code signing and notarisation for macOS and Windows distribution.
