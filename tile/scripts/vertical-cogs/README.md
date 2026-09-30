# vertical-cogs — vertical-reference COGs for the terrain pipeline

Builds and validates the rasters that turn a DEM layer into JGD2024
ellipsoidal heights without touching the DEM itself:

| Output | What it is | Source |
|---|---|---|
| `hyokorev-jgd2011-to-jgd2024/dh_bm.tif`, `dh_tr.tif` | JGD2011 → JGD2024 height correction ΔH, one raster per GSI parameter file, node for node | PatchJGD(H) `hyokorevBM_jgd2024_h.par` / `hyokorevTR_jgd2024_h.par` (Ver.1.0.0, 2025-04-01) |
| `hyokorev-jgd2011-to-jgd2024/tr_meshes.json` | the 22 secondary meshes GSI converted with TR, with provenance | GSI [data_update_info_all](https://service.gsi.go.jp/kiban/app/data_update_info_all/), 2025-07-31 entry (re-read by `fetch_sources.py`) |
| `geoid/jpgeo2024-hrefconv2024/geoid.tif` | geoid for the JGD2024 dataset | GSI `JPGEO2024+Hrefconv2024.isg` (the combined file GSI publishes; not re-derived) |
| `geoid/gsigeo2011-v2.2/geoid.tif` | geoid for the existing JGD2011 dataset | GSI `gsigeo2011_ver2_2.asc` |

Each output directory also gets a `manifest.json` (grid, conventions, source
URLs + sha256 + version header, output sha256). Nothing downloaded or
generated here is committed; `sources.json`/`manifest.json` carry provenance.

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

## What GSI did (established with `validate_oracle.py`)

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
  parameter files**: `check_fallback.py` shows that "secondary mesh reads a
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

**Not served: a single merged grid.** `build_dh.py --diagnostic-merged`
still writes `diagnostic/dh_merged_diagnostic.tif` (TR on every node a listed
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

## Running

Requires `uv`, the GDAL CLI (`gdal_translate`), and for the oracle `rclone`
with the repo's `rclone.r2.conf` plus a 基盤地図情報 login file
(`GSI_LOGIN_CONF=path`, `GSI_USER=`/`GSI_PASS=` lines; read by Python only,
never echoed). Work files go to `work/` (git-ignored) unless redirected.

```bash
cd tile/scripts/vertical-cogs
uv run test_sampler.py                                   # sampler spec tests
GSI_LOGIN_CONF=... uv run fetch_sources.py --out work/src    # BM/TR, GSIGEO2011, JPGEO2024 ISG (login)
uv run build_dh.py --src work/src --out work/out            # add --diagnostic-merged for the not-served single grid
uv run build_geoid.py jpgeo2024-hrefconv2024 --src work/src --out work/out
uv run build_geoid.py gsigeo2011-v2.2        --src work/src --out work/out
uv run validate_geoid.py work/out/geoid/jpgeo2024-hrefconv2024/geoid.tif
uv run validate_geoid.py work/out/geoid/gsigeo2011-v2.2/geoid.tif
uv run check_fallback.py --src work/src
# oracle: ≤ 6 secondary meshes; the JGD2024 zips are parsed in memory only
rclone --config ../../../rclone.r2.conf lsf \
  r2:plateau-terrain-ortho-backup/terrain/base_terrain/kibanchizu_dem_20250129/s1_geotiff_raw/dem5a/ > work/dem5a.txt   # slow, flat dir
GSI_LOGIN_CONF=... uv run validate_oracle.py --dh-dir work/out/hyokorev-jgd2011-to-jgd2024 \
  [--diagnostic-merged work/out/diagnostic/dh_merged_diagnostic.tif] \
  --listing-dem5a work/dem5a.txt --listing-dem1a work/dem1a.txt --rclone-conf ../../../rclone.r2.conf \
  473121:DEM5A:20250620 473120:DEM5A:20250620 483103:DEM5A:20250620 \
  543664:DEM5A:20250620 574037:DEM1A:20250423 664241:DEM5A:20250620
```

Pick oracle meshes whose JGD2024 edition is a pure re-issue: the pre-cutoff
edition must be ≤ 2025-01-29 (what the R2 backup holds) and the JGD2024
edition must be the 2025-04..06 re-issue, not a later survey
(`gsi.Client().dem_editions(mesh6)`).

## Uploading

Versioned, never overwritten, `rclone copyto --immutable` (an existing key
with different content is an error, not an overwrite):

```
r2:plateau-terrain/vertical/hyokorev-jgd2011-to-jgd2024/v1/{dh_bm.tif,dh_tr.tif,tr_meshes.json,manifest.json}
r2:plateau-terrain/vertical/geoid/jpgeo2024-hrefconv2024/v1/{geoid.tif,manifest.json}
r2:plateau-terrain/vertical/geoid/gsigeo2011-v2.2/v1/{geoid.tif,manifest.json}
```

Public URLs: `https://tiles.plateau.city/terrain/vertical/...` (same paths).
A changed input (new parameter version, a different TR list) is a new `v<N>`.

The terrain config Worker (`cloudflare/tiles/src/r2.ts`, `tileConfig`) lists
only `base/`, `patch/`, `sea/` for DEM datasets. Immediately before and after
uploading, fetch `https://tiles.plateau.city/terrain/config.json` and confirm
version, layer count and key set are unchanged and contain no `vertical/` key
— a ΔH raster picked up as a DEM overlay would paint ±0.4 m as terrain. Then
check each object through the public URL (HEAD `content-length`, sha256 of
the body against the manifest, and a few sampler reads via `/vsicurl/`).

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
