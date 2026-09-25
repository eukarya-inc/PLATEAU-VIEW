# Vendored i-UR code lists

The `gml:Dictionary` documents the converter writes into a converted package's
`codelists/` directory, so a code cited through a `codeSpace` resolves inside
the package the way it resolves in a PLATEAU package, through the relative
`../../codelists/<file>.xml`.

One directory per i-UR minor version, mirroring `../schemas/iur/`.

| Directory | Contents | Embedded |
| --- | --- | --- |
| `4.0/` | The 315 published i-UR 4.0 lists, the complete set. | Yes, as `CODELISTS_4_0`. |

`core/build.rs` embeds every `*.xml` under `4.0/` and nothing else, so a
sibling minor added here for reference is not shipped. Adding one to the output
means naming it in `build.rs`.

## Provenance

Copied unchanged from `https://www.geospatial.jp/iur/codelists/4.0/`, the set as
published on 2026-03-25, and compiled into the binary, so conversion never
touches the network.

At the time of writing, all 315 published file names are exactly the same, and
every file is byte-identical to the published copy.

Drift is expected eventually. A code list carries no version of its own, the
`gml:Dictionary` root holding only `xsi:schemaLocation` and `gml:id`, so the
directory name is the only record of which minor these are. The published minors
are edited in place after release, the published `3.0/` carrying files dated
2023 through 2025 and `3.2/` carrying edits a month after its release, so the
4.0 set can move under us too.

The lists replace the input's copies of the same file names. A list the input
authors itself, matched by the profile's `[codelists] local` patterns, and a
list with no published counterpart, are kept as shipped.

Every file is a GML 3.1.1 `gml:Dictionary` against the SimpleDictionary profile,
which is the form both the input lists and the published 4.0 lists take.