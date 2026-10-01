# vertical-cogs — vertical-reference COGs for the terrain pipeline

Builds and validates the rasters that turn a DEM layer into JGD2024
ellipsoidal heights without touching the DEM itself:

| Output | What it is | Source |
|---|---|---|
| `hyokorev-jgd2011-to-jgd2024/dh_bm.tif`, `dh_tr.tif` | JGD2011 → JGD2024 height correction ΔH, one raster per GSI parameter file, node for node | PatchJGD(H) `hyokorevBM_jgd2024_h.par` / `hyokorevTR_jgd2024_h.par` (Ver.1.0.0, 2025-04-01) |
| `hyokorev-jgd2011-to-jgd2024/tr_meshes.json` | the 22 secondary meshes GSI converted with TR, with provenance | GSI [data_update_info_all](https://service.gsi.go.jp/kiban/app/data_update_info_all/), 2025-07-31 entry (re-read by `vcog.py fetch`) |
| `geoid/jpgeo2024-hrefconv2024/geoid.tif` | geoid for the JGD2024 dataset | GSI `JPGEO2024+Hrefconv2024.isg` (the combined file GSI publishes; not re-derived) |
| `geoid/gsigeo2011-v2.2/geoid.tif` | geoid for the existing JGD2011 dataset | GSI `gsigeo2011_ver2_2.asc` |

The products are declared in `products.toml` and driven by one entrypoint,
`vcog.py` (fetch → build → validate → publish → verify). Each product version
also gets a `manifest.json` (grid, conventions, source URLs + sha256 + version
header, output sha256). Nothing downloaded or generated here is committed;
`sources.json`/`manifest.json` carry provenance.

## Conventions (all rasters)

- **Node-centred grid.** GSI's values live on grid *nodes* (ΔH: the
  south-west corner of each 30″ × 45″ tertiary mesh; geoids: the 1′ × 1.5′
  lattice nodes). The geotransform origin is set half a cell north-west of
  the north-west node, so every **pixel centre is a node**. A reader using the
  ordinary PixelIsArea convention and interpolating bilinearly between pixel
  centres therefore reproduces GSI's node-bilinear interpolation exactly.
  Do not "fix" the half-cell offset. The same statement is in each COG's
  `VREF_GRID` metadata.
- EPSG:6668, float32, **nodata = NaN** (chosen over −9999 so a reader that
  forgets the nodata check poisons the result instead of adding −9999 m).
- COG: ZSTD + PREDICTOR=3, 512 px blocks, nearest-neighbour overviews (the
  repo's DEM recipe). Overviews are for viewing only — samplers read the
  full-resolution IFD.
- Signs: `H_jgd2024 = H_jgd2011 + ΔH`; `h_ellipsoidal = H + geoid`.

## Sampling semantics (normative: `sampler.py`)

`sampler.py` is the reference the tile server (P2) and the Worker must match;
its docstring is the spec and `test_sampler.py` pins every rule:

1. `fx = (lon − x0)/dx − 0.5`, `fy = (y0 − lat)/dy − 0.5` in float64, snapped
   to the nearest integer when within 1e-9 of it (needed because 1/80°, 1/120°
   are not representable; < 0.01 mm effect).
2. Bilinear between the 4 surrounding pixel centres; a neighbour whose weight
   is exactly 0 is not used (so points on node rows/columns work at the grid
   edge and next to nodata — same as GSI's gsigeome and the `japan-geoid` crate).
3. NaN if any *used* neighbour is nodata or outside the raster. No clamping,
   no edge extension, no renormalisation over the valid neighbours.

ΔH is never sampled from one grid alone: the served form is
`dh_bm.tif` + `dh_tr.tif` + `tr_meshes.json` combined by the selection rule
below (`sampler.DhGsi`; normative text in the product's
`manifest.json` → `selection_rule`).

## What GSI did (established with `vcog.py validate --oracle`)

GSI re-issued its DEM1A/5A/5B/5C on 2025-07-31 as `round(H_2011 + ΔH, 2)`.
Comparing GSI's JGD2024 files with the JGD2011 originals on six secondary
meshes shows the rule is:

```
ΔH(p) = TR(p)   if p lies in a secondary mesh on GSI's TR list (22 meshes)
      = BM(p)   else, if all 4 BM nodes of p's cell exist
      = TR(p)   else                       (per-cell fallback)
```

- The TR list is GSI's published list (data_update_info_all, 2025-07-31),
  served as `tr_meshes.json`:
  473113, 473121–23, 473131–33, 473141–43, 473151–53, 473161–63, 473173,
  543664, 543665, 543674, 543675, 553605. It is **not derivable from the
  parameter files**: `vcog.py validate` (`check_fallback.py`) shows that "secondary mesh reads a
  node missing from BM but present in TR" yields all 22 plus 28 more; 26 of
  those have no DEM at all, 483104's gap cells have no DEM pixels, but
  483103's gap cell (48310309) has 8,605 land pixels and GSI did *not* list
  483103 — it converted that one cell with TR and the rest of 483103 with BM.
  Meanwhile 473121, whose only BM gap is also a single cell (47312199), was
  converted with TR throughout. So the list is GSI's own (inferred:
  administrative, around the areas the BM header names) and is used as data.
- Because the result is discontinuous along the edges of TR meshes and of
  per-cell fallbacks, **no single node grid can reproduce it**: a node shared
  by a BM cell and a TR cell would need two values.

## Status: served form of ΔH (decided)

**Served: `dh_bm.tif` + `dh_tr.tif` + `tr_meshes.json`**, combined per point:

1. Secondary mesh of p: `i5 = floor(lat·12)`, `j75 = floor((lon−100)·8)`,
   `mesh6 = (i5 div 8)·10000 + (j75 div 8)·100 + (i5 mod 8)·10 + (j75 mod 8)`.
   A point exactly on a secondary-mesh boundary belongs to the mesh to its
   north/east.
2. `mesh6` listed in `tr_meshes.json` → `ΔH = sample(dh_tr)` (NaN stays NaN;
   no fallback to BM).
3. Otherwise `ΔH = sample(dh_bm)` if that is not NaN.
4. Otherwise ("BM out of range") → `ΔH = sample(dh_tr)`.

- **"BM out of range"** means `sample(dh_bm)` is NaN: *any* corner node with a
  non-zero bilinear weight is missing (for a point inside a tertiary cell:
  any of the 4 corners, not all 4; on a cell edge only that edge's 2 nodes).
  For interior points this is exactly a per-tertiary-cell fallback, which is
  what GSI did in 483103 (cell 48310309, SE corner 48310400 absent from BM).
- **Shared nodes:** nothing is merged. Each point reads all four corners from
  one grid, so a node shared by a BM-evaluated cell and a TR-evaluated cell
  gives its BM value to one and its TR value to the other. ΔH is therefore
  discontinuous along those cell edges (e.g. 473120 | 473121, up to 33 mm) —
  intended, because GSI's DEMs are.

**Not served: a single merged grid.** `vcog.py build --diagnostic-merged`
still writes `work/out/diagnostic/<id>/merged.tif` (TR on every node a listed
mesh reads, else BM, else TR) only so its error stays measurable. It cannot
reproduce GSI: it has 238 conflict nodes (BM ≠ TR, up to 33 mm) and fails the
oracle on BM meshes bordering the list (473120: 92.98 % within 5 mm, max
17.9 mm; 483103: 92.88 %, max 26.9 mm).

**Known, unexplained residuals** (the served form still meets the ≥ 99 %
criterion everywhere):

- 543664: tertiaries 5436-64-73 and 5436-64-94 (958 px, next to the BM gap)
  are 5.8–7.6 mm off on every pixel, i.e. 1–2.5 mm beyond rounding. The
  published TR file does not explain it.
- 574037 (DEM1A): 26 pixels valid in the 2011 edition (≈ 4.1 m) are nodata
  in GSI's JGD2024 edition — GSI dropped them.

## Running the pipeline

Requires `uv` and the GDAL CLI (`gdal_translate`); `publish`/`verify` and the
oracle also need `rclone` with the repo's `rclone.r2.conf` (remote `r2`), and
login-only sources need a 基盤地図情報 login file (`GSI_LOGIN_CONF=path`,
`GSI_USER=`/`GSI_PASS=` lines; read by Python only, never echoed or stored).
Work files go to `./work` (git-ignored) unless `--work` says otherwise.

```bash
cd tile/scripts/vertical-cogs
uv run vcog.py test                              # sampler spec + publish guard tests
uv run vcog.py list
GSI_LOGIN_CONF=... uv run vcog.py fetch all      # -> work/src/<id>/ + sources.json
uv run vcog.py build all                         # -> work/out/<key_prefix>/<version>/ (+ --diagnostic-merged)
uv run vcog.py validate all                      # geoids vs japan-geoid crate + GSI calculator; mesh-list check
GSI_LOGIN_CONF=... uv run vcog.py validate hyokorev-jgd2011-to-jgd2024 --oracle \
  --listing-dem5a work/dem5a.txt --listing-dem1a work/dem1a.txt
uv run vcog.py publish all --dry-run             # read-only: key/manifest guards, existence, config.json
uv run vcog.py publish <id> --sample             # the real upload (see guarantees below)
uv run vcog.py verify all --sample               # re-check published objects against the local build
```

`validate` exits non-zero on failure: geoids must agree with the crate to
< 0.1 mm with no coverage disagreement and with GSI's calculator to its
0.1 mm print precision; each oracle mesh must be within 5 mm on ≥ 99 % of
pixels.

The oracle downloads GSI's JGD2024 DEM for at most 6 secondary meshes (the
zips are parsed in memory; only parsed arrays are cached under
`work/oracle/`). The JGD2011 side comes from the R2 backup, whose `dem5a`
directory is huge and flat: list it once and pass the listing:
`rclone --config ../../../rclone.r2.conf lsf r2:plateau-terrain-ortho-backup/terrain/base_terrain/kibanchizu_dem_20250129/s1_geotiff_raw/dem5a/ > work/dem5a.txt`.
Pick oracle meshes whose JGD2024 edition is a pure re-issue: the pre-cutoff
edition must be ≤ 2025-01-29 (what the backup holds) and the JGD2024 edition
the 2025-04..06 re-issue, not a later survey (`gsi.Client().dem_editions(mesh6)`).

## Product file (`products.toml`)

`[defaults]` holds the storage and URLs (`remote`, `rclone_config`, `bucket`,
`allowed_prefix`, `public_base`, `config_url`); every one can be overridden on
the `publish`/`verify` command line. Each `[[product]]` has:

| field | meaning |
|---|---|
| `id` | name used on the command line and in work paths |
| `kind` | `height-correction` or `geoid` |
| `version` | published as `<key_prefix>/<version>/<file>`; never reused |
| `key_prefix` | must sit under `allowed_prefix` (`vertical/`) |
| `from`, `to` | (height-correction) datum pair, e.g. `jgd2011` → `jgd2024` |
| `[[product.grid]]` | (height-correction) one per PatchJGD(H) `.par`: `name`, output `file`, `content` text, `source` {`url`, `member` in the zip, `page`, `note`} |
| `[product.selection]` | (height-correction, optional) `kind = "listed-meshes"` with grid roles `primary`/`listed`/`fallback`, the `list_file` to write, the GSI `page` + `anchor` the list is scraped from, and the reviewed `expected` list — build stops if GSI's page says anything else |
| `[product.oracle]` | (height-correction, optional) `meshes = ["mesh6:TYPE:edition", ...]`, ≤ 6 |
| `format` | (geoid) `isg` (ISG 2.0) or `gsigeo-asc` |
| `[product.source]` | (geoid) public `url` + `member`, or `login = true` + `catalogue` {`version`, `type`} for the 基盤地図情報 service |
| `[product.validate]` | (geoid) `crate_loader` (a `japan_geoid.load_embedded_*` name), GSI `calculator` URL and the JSON `calculator_key` to compare |

### Adding a product

- **Another PatchJGD(H) parameter** (e.g. `noto2024_02h.par`): add a
  `height-correction` product with one `[[product.grid]]` and no
  `[product.selection]`; the manifest rule is then simply `sample(<file>)`.
  Oracle validation only applies if GSI published DEMs converted with it.
- **A new geoid version**: add a `geoid` product with its `format`, source and
  `crate_loader` (if the crate embeds it; otherwise validation needs a new
  reference).
- **A changed input** for an existing product (new `.par` version, a different
  mesh list): bump `version`; never edit a published version.

Then `fetch` → `build` → `validate` → `publish --dry-run` → `publish` → `verify`.

## Publishing: what `publish` guarantees

`publish.py` encodes the procedure used for v1; nothing is uploaded unless
every check passes, and a failure after the upload removes exactly the
objects that run uploaded:

1. **Keys stay out of the terrain stack.** Every key must be under
   `allowed_prefix` (`vertical/`); keys under the prefixes the DEM config
   Worker enumerates (`DEM_CONFIG_PREFIXES` = `base/`, `patch/`, `sea/`, from
   `tileConfig` in `cloudflare/tiles/src/r2.ts`; a test pins the two together)
   are refused, as is an allowed prefix overlapping them, absolute keys and `..`.
2. **The set is what the manifest says.** The version directory must contain
   `manifest.json` and exactly the files it lists, with matching sha256/size.
3. **Never overwrite.** Each key's existence is checked first (by listing its
   parent — `rclone lsjson --stat` reports a missing S3 object as a directory
   with exit 0); any existing key aborts, also in `--dry-run`. Uploads also
   use `rclone copyto --immutable`.
4. **config.json is unchanged.** It is fetched (cache-busted) immediately
   before and after. If an uploaded key, or anything under `allowed_prefix`,
   appears as a layer, or version / layer count / key-set hash changed at
   all, the upload is rolled back and publish exits 3.
5. **Public verification.** Each object is fetched via `public_base`: HEAD
   `content-length`, and the sha256 of the body must match the manifest;
   `--sample` also reads each COG through GDAL `/vsicurl/` and requires the
   reference sampler to return bit-identical values to the local file.
   Failure rolls back and exits 4.

`--dry-run` only reads (existence checks and config.json) and prints the plan.
All HTTP requests send a browser-like User-Agent: Cloudflare in front of
`tiles.plateau.city` answers 403 to Python's default one.

## Published objects

Bucket `plateau-terrain`, public at `https://tiles.plateau.city/terrain/<key>`:

| key | bytes | sha256 |
|---|---|---|
| vertical/hyokorev-jgd2011-to-jgd2024/v1/dh_bm.tif | 2,363,601 | a439f9e2a1337f400550a82a01e5c639e2a78502925324ce3eceb82e2e492ee8 |
| vertical/hyokorev-jgd2011-to-jgd2024/v1/dh_tr.tif | 2,577,201 | a857ef9ccaf201043b37a5a57bfeee9a85d4b2c1c40e9712a51b27233f1a3d58 |
| vertical/hyokorev-jgd2011-to-jgd2024/v1/tr_meshes.json | 1,399 | 0289b2421d07b8d11b30c079b2198814a2b40b75002fb84b13eea03836179d43 |
| vertical/hyokorev-jgd2011-to-jgd2024/v1/manifest.json | 6,504 | 56493e8f562c61094ea3f5b49b99a4d67b86a54b6106b6c90e4b55f1bceb142e |
| vertical/geoid/jpgeo2024-hrefconv2024/v1/geoid.tif | 728,563 | 700fa97558eeef77de70a677328ec955228614d5d134aea96ae180ab3aaa67a7 |
| vertical/geoid/jpgeo2024-hrefconv2024/v1/manifest.json | 3,316 | ba5645705911e17186af347ef4b881350b8f9bd15b501f3da8464610134edfd6 |
| vertical/geoid/gsigeo2011-v2.2/v1/geoid.tif | 528,254 | 88fb79d103a8fbc15b35754b0fcec4ba6ff79d07d01c3e1c85fbbd089878a1bb |
| vertical/geoid/gsigeo2011-v2.2/v1/manifest.json | 1,672 | a8ed58e281cd75317cb1eccf53752607d0d3d7936848c2fcf081365d350d4e0f |

A fresh `fetch` + `build` reproduces all four COGs byte for byte. The JSON
files do not: they record the fetch time, and v1 was published before the
pipeline was generalised, so its hyokorev manifest has no machine-readable
`selection` block (it names the files in `tr_meshes` / `selection_rule`
text) and its geoid manifests list the COG under `cog` instead of `files`.
`sampler.DhGsi.open` falls back to the v1 file names. `verify` therefore
reports the v1 JSON files as different from a new build; that is expected.

## Validation results (2026-10-01)

Geoids, 5,000 random points in coverage + every valid node, vs `japan-geoid`
0.6.0 (the crate the tile server links):

| model | max \|Δ\| random | max \|Δ\| nodes | coverage disagreements |
|---|---|---|---|
| jpgeo2024-hrefconv2024 | 1.9 µm | 1.9 µm | 0 (741 edge nodes NaN in the crate only: its float arithmetic pulls in the nodata neighbour at exact nodes) |
| gsigeo2011-v2.2 | 1.8 µm | 1.9 µm | 0 (1,685 such edge nodes) |

GSI geoid calculator (7 points incl. Naha, Hachijo-jima, Chichi-jima where
Hrefconv ≠ 0): all within 0.05 mm (the calculator prints 0.1 mm).

ΔH oracle (fraction of pixels with |2011 + ΔH − 2024| ≤ 5 mm; float32 storage
adds up to ~0.05 mm):

| mesh | type | on TR list | **served (`DhGsi`)** | BM only | TR only | merged diagnostic |
|---|---|---|---|---|---|---|
| 473121 | DEM5A | yes | 99.97 % | 0 % | 99.97 % | 99.97 % |
| 473120 | DEM5A | no (W of 473121) | 99.98 % | 99.98 % | 0 % | 92.98 % |
| 483103 | DEM5A | no (gap cell) | 99.996 % | 99.997 %¹ | 8.3 % | 92.88 % |
| 543664 | DEM5A | yes | 99.95 % (max 7.6 mm) | 0.6 %¹ | 99.95 % | 99.95 % |
| 574037 | DEM1A | no | 100 % | 100 % | 0 % | 100 % |
| 664241 | DEM5A | no | 100 % | 100 % | 0 % | 100 % |

The served form is within 10 mm on 100 % of pixels of all six meshes.

¹ over the pixels where BM is defined.
