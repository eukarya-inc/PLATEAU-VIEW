# dem-cogs — the DEM COGs of the terrain stack

Builds, verifies and publishes the Cloud Optimized GeoTIFFs that the terrain
Worker stacks into the `plateau-terrain-experimental` DEM source:

| Prefix (bucket `plateau-terrain`) | COGs | Content |
|---|---|---|
| `base/dem10/<mesh4>.tif`, `<mesh4>-fill.tif` | 176 + 21 | GSI 基盤地図情報 DEM10B (1:25,000 contours) and DEM10A (volcano base maps), 1/9000° |
| `base/dem5/<mesh4>.tif` | 135 | DEM5A (airborne laser), DEM5B, DEM5C (photogrammetry), 1/18000° |
| `base/dem1/<mesh4>.tif` | 33 | DEM1A, 1/90000° |
| `patch/<group>/<name>.tif` | 15 | local-government DEMs (Noto, Tokyo, Shizuoka, four cities) |
| `sea/` | 0 | enumerated by the Worker, currently empty |

Public at `https://tiles.plateau.city/terrain/<key>`. `<mesh4>` is a primary
mesh code (40′ × 1°). Everything is orthometric height in metres, Float32,
NoData −9999; base COGs keep the source's geographic CRS (EPSG:4612 JGD2000
or EPSG:6668 JGD2011).

This directory is the canonical home of the pipeline. It replaces the
scripts that produced the served stack from a scratch area (the batch
`tmp/tile-scripts/cog-batch/convert-cogs.sh` and the one-off gap-fill and
validation scripts); those are left where they are, but changes go here.

## Quick start

Requires `uv`, the GDAL CLI (`gdalbuildvrt`, `gdal_translate`, `gdalinfo`,
`gdalsrsinfo`) and, for anything touching R2, `rclone` with the repo's
`rclone.r2.conf` (remote `r2`; `--rclone-config` or `$RCLONE_CONFIG_PATH`).
Work files go to `./work` (git-ignored) unless `--work` says otherwise.

```bash
cd tile/scripts/dem-cogs
uv run demcog.py test                                   # 68 tests, no network (GDAL CLI needed)

# sources -- either from GSI (login) ...
GSI_LOGIN_CONF=~/gsi_login.conf uv run demcog.py fetch dem10 4730 --before 2025-04-01
uv run demcog.py gml2tif                                # work/zips/* -> work/src/<product>/*.tif + provenance.jsonl
# ... or from the 2025-01-29 backup the served stack was built from
uv run demcog.py list-backup all                        # once: work/listings/<product>.json
uv run demcog.py mirror dem10 4730

uv run demcog.py build dem10 4730 --as-served           # work/out/base/dem10/4730.tif (+ -fill.tif) + manifests
uv run demcog.py labels --mesh 473067                   # datum label per secondary mesh, from the manifests

# verification against what is served (read-only)
uv run demcog.py strict dem10 all                       # every source pixel vs the served stack
uv run demcog.py reproduce base/dem10/4730-fill.tif     # rebuild one served COG, compare pixels

# publishing (dry run unless --execute; see "Publishing")
uv run demcog.py publish base/dem10/9999.tif
```

The GSI login file holds `GSI_USER=` / `GSI_PASS=` lines; it is read by
Python only (`../vertical-cogs/gsi.py`), never printed, logged or stored.

## Recipe

Per primary mesh and CRS group (`build.py`):

```bash
gdalbuildvrt -srcnodata -9999 -vrtnodata -9999 -input_file_list <grids, bottom -> top> m.vrt
gdal_translate -of COG -ot Float32 m.vrt out.tif -co BLOCKSIZE=512 -co RESAMPLING=nearest \
  -co COMPRESS=ZSTD -co PREDICTOR=3 -co NUM_THREADS=ALL_CPUS -co BIGTIFF=YES
```

BigTIFF, ZSTD with the floating-point predictor, 512 px blocks, Float32,
NoData −9999, the source CRS (not reprojected), the source spacing, and
GDAL's default overviews (nearest, halving until smaller than one block, e.g.
9000×6000 → 4500, 2250, 1125, 562, 281). `BLOCKSIZE=512` and `-ot Float32`
are the defaults and change nothing; they are written out to make the recipe
explicit. After writing, `build` re-reads the COG and refuses it if layout,
compression, predictor, type, block size, NoData or EPSG differ.

Per-grid GeoTIFFs (`fggml.py`, `gml2tif`): one per FG-GML XML (secondary mesh
for DEM10, tertiary mesh for DEM5/DEM1), float32, NoData −9999, the XML's
`gml:Envelope` as the extent (pixel edges), `gml:startPoint` and a short
`gml:tupleList` honoured, `sequenceRule` must be `+x-y`, EPSG 4612/6668 from
`srsName` (`fguuid:jgd2000.bl` / `fguuid:jgd2011.bl`; `jgd2024.bl` is
refused, see caveats). Named like the XML (`FG-GML-4730-67-DEM10B-20161001.tif`),
which is also the layout of the backup's FME conversion. For 21 DEM10B grids
this conversion was compared with the backup's FME output: same CRS,
geotransform, mask and values.

Patches (`patch.py`, port of `_patch_region`): every `.tif`/`.asc` in the
source directory, NoData and CRS checked to be uniform across *all* inputs
(refused otherwise), NoData mapped to −9999, geographic CRS kept with the
recipe above, projected CRS warped to EPSG:3857 `GoogleMapsCompatible` with
256 px blocks. The source directories are listed in `stacks.toml`.

## Rules the builder enforces (`plan.py`)

Each rule exists because the original batch got it wrong without noticing.

1. **No silently dropped input.** `gdalbuildvrt` keeps the first input's CRS
   and *drops every input with another CRS*, printing a warning and exiting 0.
   The original batch discarded that output, so whole DEM10B secondary meshes
   went missing from the served COGs. `build` refuses any `gdalbuildvrt`
   warning and checks that the VRT lists every input, in order.
2. **CRS split.** The grids of a primary mesh are grouped by EPSG; the larger
   group becomes `<mesh4>.tif`, the other `<mesh4>-fill.tif` (tie: the newer
   datum is the main COG). A third or unknown CRS is refused. `--main-crs
   3623=6668,...` pins the main group; `--as-served` applies the pins of the
   served stack (`served/state.json`: 3623, 4329, 6545, 6645 have the smaller
   group as their main COG, because the original batch named the main COG
   after whatever CRS `find` listed first).
3. **Explicit paint order.** Inside a COG the grids are listed bottom → top
   by product — DEM10B < DEM10A, DEM5C < DEM5B < DEM5A (`stacks.toml`) — then
   by mesh code, so the better product wins where two cover a pixel.
4. **Cross-COG order.** The Worker paints `<mesh4>-fill.tif` under
   `<mesh4>.tif` (`demPriority`, then key order: `-` < `.`). If a mesh's
   better product would land in the fill and a worse one in the main COG,
   the build is refused.
5. One grid per (product, mesh); pixel spacing, NoData and type are checked
   on every input.

## Manifest (`<key>.manifest.json`)

Written next to every COG by `build` (and published to
`manifests/<key>.manifest.json`):

- `key`, `stack`, `primary`, `role` (main/fill), `epsg`, `label`, `bytes`,
  `sha256`, `md5`, the checked COG properties, `paint_order`, the exact
  `recipe`, the `gdal` version, the `tool` git revision, `built_at`;
- `grids`: every input in VRT order — `name`, `mesh`, `product`, `edition`
  (作成年月日 from the file name), `label`, `epsg`, `bytes`, `sha256`, and,
  for grids made by `gml2tif`, `origin` (`zip`, `zip_sha256`, `xml`,
  `xml_sha256`, `srs_name`, `dev_date`, GSI catalogue `gsi_id`);
- `secondary_meshes`: `{mesh6: {product: {labels: {jgd2000: n, ...},
  editions: [...], grids: n}}}` — the datum label of every secondary mesh,
  which is what the JGD2024 height correction needs.

`labels [--mesh <prefix>] [manifests...]` prints that table;
`labels --served` queries `served/dem10-labels.tsv` instead (below).

## Verification

**`strict <stack> <mesh4,...|all>`** reads every source grid (from the backup
via `/vsis3/`, or `--src DIR`) and the served COGs of its primary at full
resolution over the public URL, paints them like the Worker (fill under
main), and classifies every pixel valid in at least one product: reproduced
(within 0.01 m of a product valid there), missing (served stack has no
data — a dropped grid) or wrong. A mesh fails on any missing or wrong pixel.
It also counts *order deviations*: pixels where two products differ and the
served value is the lower product's. `--exclude KEY` leaves a served COG out.

**`reproduce <key>...`** downloads a served COG, rebuilds it from the sources
(the served file's CRS decides which group and role is rebuilt), and compares
bytes, grid, overviews and every full-resolution pixel. Differing pixels are
attributed per mesh: `order` (the served value is another product's value
there), `served_nodata`, `rebuilt_nodata`, `other`. **Bytes match only with
the same GDAL version** (the original batch ran an older GDAL, which e.g.
did not write `OVERVIEW_RESAMPLING` metadata), so pixels are the criterion.
`patch/...` keys are rebuilt from their `stacks.toml` source directory.

### Results (2026-10-02, GDAL 3.12.0)

`strict dem10 all` — 4,885 meshes (177 primaries), **all pass**:
3,132,652,422 source pixels, 0 missing, 0 wrong. 96 meshes carry both DEM10A
and DEM10B; in **42 of them the served COG paints DEM10B over DEM10A**
(7,072,101 px). With `--exclude` of the two fills uploaded last
(`4730-fill.tif`, `4931-fill.tif`) the same check fails exactly the two
meshes they fixed: 473067 (647,964 of 843,750 px missing) and 493107
(13,293 of 13,293).

`strict dem5 4829,4631` — 442 meshes pass (9,316,485 px); 59 of 116
multi-product meshes deviate from 5C < 5B < 5A (101,672 px).
`strict dem1 5239,5340,4931` — 143 meshes pass (120,656,250 px); DEM1A has
no overlapping product.

`inventory all` (header of every backup grid): DEM10B 4,690 `jgd2000` +
195 `jgd2011`; DEM10A 96 `jgd2000`; DEM5A 287,681, DEM5B 18,076, DEM5C
21,616, DEM1A 16,317 all `jgd2011`; no unreadable grid. Only the 21 DEM10
primaries below mix labels — DEM5 and DEM1 lost nothing to the CRS skip
(`served/backup-labels-summary.json`).

`reproduce`:

| served key | rebuilt from | bytes | pixels | different | attribution |
|---|---|---|---|---|---|
| `base/dem10/4730-fill.tif` | backup | identical | 843,750 | 0 | |
| `base/dem10/3926-fill.tif` | GSI (`fetch --as-listed`) | identical | 843,750 | 0 | |
| `base/dem10/3926.tif` | GSI (`fetch --as-listed`) | differ | 5,062,500 | 0 | |
| `base/dem10/5339.tif` | backup | differ | 54,000,000 | 0 | |
| `base/dem10/4730.tif` | backup | differ | 54,000,000 | 144,443 | all `order` (DEM10A/B) |
| `base/dem5/5139.tif` | GSI (`fetch --before 2025-04-01`) | identical | 83,025,000 | 0 | |
| `base/dem5/5942.tif` | backup | differ | 9,315,000 | 0 | |
| `base/dem5/6641.tif` | backup | differ | 19,743,750 | 0 | |
| `base/dem5/4829.tif` | backup | differ | 51,840,000 | 86,444 | all `order` (DEM5A/B/C) |
| `base/dem5/4631.tif` | backup | differ | 29,700,000 | 24,555 | all `order` (DEM5A/B) |
| `base/dem1/5340.tif` | backup | differ | 8,437,500 | 0 | |
| `patch/others/susami-cho.tif` | backup | identical | 4,252,500 | 0 | |
| `patch/others/tamana-shi.tif` | backup | differ | 34,603,008 | 0 | (geotransform differs by 1.9e-9 m) |

So the served stack is reproducible pixel for pixel wherever products do not
overlap; where they do, the rebuild applies the declared order and differs
from the served files only on those pixels.

## Served state (`served/`)

What is in the bucket now and where each part came from; checked against the
bucket listing on 2026-10-02 (config.json version `terrain-380-fc530cba`,
380 layers).

- `objects.tsv` — every served COG: key, bytes, MD5 (as `rclone lsjson --hash`
  reports it).
- `dem10-labels.tsv` — every DEM10 grid of the backup (4,885 DEM10B + 96
  DEM10A): secondary mesh, product, edition, datum label, and the served COG
  holding it. `demcog.py labels --served --mesh 4730`.
- `backup-labels-summary.json` — label counts per product and the 21 mixed
  primaries (output of `inventory all`).
- `state.json` — the main-CRS pins of the served dem10 stack.
- `dem10-fills.json` — the 21 `-fill` COGs (below).
- `dem5-refresh.json` — `base/dem5/4730.tif` and `5139.tif` (below).

Provenance of the served base COGs:

1. **Original batch** (2025, `convert-cogs.sh` on a GCE VM, from the backup
   `plateau-terrain-ortho-backup/terrain/base_terrain/kibanchizu_dem_20250129/s1_geotiff_raw/`,
   i.e. the GSI download of 2025-01-29 converted by FME): the 176 main
   `base/dem10` COGs, 133 `base/dem5` COGs, all 33 `base/dem1` COGs. VRT
   order = `find` order (arbitrary), and grids in the minority CRS of a
   primary were dropped silently.
2. **dem5 refresh** (2026-09-06): `base/dem5/4730.tif` (Sakurajima /
   Kirishima void fill) and `base/dem5/5139.tif` (Miyake-jima caldera) were
   rebuilt from GSI downloads — per secondary mesh the newest edition created
   before 2025-04-01 (JGD2011), order DEM5C < DEM5B < DEM5A; 104 + 12 zips,
   names and sha256 in `dem5-refresh.json`. These two are therefore *not*
   reproducible from the backup; `fetch dem5 5139 --before 2025-04-01`
   reproduces 5139 byte for byte.
3. **dem10 fills** (2026-10-01/02): the grids the original batch dropped,
   rebuilt from GSI zips (19 COGs, 67 grids) and from the backup's
   `downloads/dem10/FG-GML-kyushu_okinawa-DEM10-Z001/` zips (4730-fill,
   4931-fill), one COG per primary in the minority CRS:

| key | bytes | label | secondary meshes | sha256 |
|---|---|---|---|---|
| `base/dem10/3623-fill.tif` | 5,712,171 | jgd2000 | 362306, 362324, 362327, 362335, 362336, 362337, 362345, 362346, 362347, 362356 | `765af06d3c0bd723b5539b744d1e2488f7beb1dd8a59b5cffec9f44173b2682e` |
| `base/dem10/3926-fill.tif` | 4,577 | jgd2011 | 392676 | `6cfc99731f5019fdca47aef55adad82961494ae029bb38738419d83db4d9e7c1` |
| `base/dem10/4042-fill.tif` | 12,283 | jgd2011 | 404200 | `31c19f432f2e7a2edfbed7a31c1feb58972d49a2d46fcec5e78ca39887b85e0d` |
| `base/dem10/4329-fill.tif` | 191,862 | jgd2000 | 432910, 432916, 432920, 432951 | `78a29757b6bcfdf3794ef832ee4923dd7af44dea640bc0b7da879273a0104a67` |
| `base/dem10/4429-fill.tif` | 108,333 | jgd2011 | 442964 | `11f13b01c516b49b9e47fbdbebd8e8a2c1e4f9caa37389bf10ac5dd8235a9879` |
| `base/dem10/4530-fill.tif` | 5,064 | jgd2011 | 453000 | `3ae4966748c702a3b2e76fe99178c581e268df6c063c818586d9fb6acd72bacc` |
| `base/dem10/4629-fill.tif` | 46,468 | jgd2011 | 462913, 462923, 462975 | `d1c3b268660f7426f860fc3280a271b050f5ded0f1cc5217b235c346600dde9b` |
| `base/dem10/4729-fill.tif` | 4,991 | jgd2011 | 472915 | `2e49b049d3989e8c269427b7d07e925be824890847902d7c37e6f977f7b9c631` |
| `base/dem10/4730-fill.tif` | 1,791,365 | jgd2011 | 473067 | `513a6c1166f581ccd1419618a49a3c913f9f4d7e06ab89654919cc01e72ab068` |
| `base/dem10/4828-fill.tif` | 126,595 | jgd2011 | 482802, 482803, 482820 | `b7dab67543ebfe5537edda894019632e85f2ac7af489327bd01ea0654f7b7d18` |
| `base/dem10/4928-fill.tif` | 3,451 | jgd2011 | 492866 | `3008fa4783f87b5b2944b9e0701e15f2444aa93cb7259a9887c36f13b2c080b4` |
| `base/dem10/4931-fill.tif` | 57,825 | jgd2011 | 493107 | `4162a4f54ce57a2f0895fad9f684b817d3f7a0e38c90269439475bd36375b017` |
| `base/dem10/5030-fill.tif` | 93,755 | jgd2011 | 503030, 503041 | `fec9842c551ec137571b16689c042cf2fb3b8bc6de81312d04546cc2128473d9` |
| `base/dem10/5032-fill.tif` | 15,569 | jgd2011 | 503243 | `99250baa8cbb4e1656dc55fb0bbb0f89f11af691e0a1f2b1c4492faf5f8af1af` |
| `base/dem10/5130-fill.tif` | 32,365 | jgd2011 | 513020, 513030 | `c90f71d90846359d996a8891b5f3994a0d5c8000c84366905afc8c3aec7a9e94` |
| `base/dem10/5132-fill.tif` | 279,224 | jgd2011 | 513203, 513204 | `5da0d1f00440644a84fab96332e1251b29920ceda36653758d4327f4bf509163` |
| `base/dem10/5133-fill.tif` | 10,032 | jgd2011 | 513313 | `bd359821f99b192afedfb23ca230592da9babb404fad06fc80446deee17b2e41` |
| `base/dem10/6039-fill.tif` | 4,047 | jgd2011 | 603963, 603964 | `cd874a52ca43d57e4ca3a1acf01a3b142e5e676ac5112ba1cc0766ee6a43b1ef` |
| `base/dem10/6440-fill.tif` | 1,552,197 | jgd2011 | 644016 | `fa4b9393b8ef5bf8df719c495c5cc9ad05aa00aff03b954d907b3a7d4051c3e2` |
| `base/dem10/6545-fill.tif` | 16,282,355 | jgd2000 | 654500, 654501, 654502, 654504, 654505, 654510, 654511, 654512, 654520, 654521, 654522, 654530, 654531, 654532, 654540, 654541, 654550, 654560, 654570, 654571 | `52917d938877ace2a3ed8d7bfc92c518ebcd960fa3bc36d79964bb2ea9148405` |
| `base/dem10/6645-fill.tif` | 9,427,262 | jgd2000 | 664500, 664501, 664510, 664511, 664512, 664521, 664522, 664531, 664532, 664542 | `a8c73c637b3c46565ca2f8c6a6e1a373f5586de69b0dfe068ee4491c821043d2` |

All 69 grids are edition 2016-10-01 except 503030, 513204, 513313
(2020-04-09), 503243 (2018-11-14), 513020, 513030 (2017-08-04); per-grid
zip, XML, srsName and zip sha256 are in `dem10-fills.json`. Bucket MD5s
match the recorded ones for all 21.

## Data caveats

- **Datum labels are per secondary mesh.** DEM10B is labelled `jgd2000` on
  4,690 secondary meshes and `jgd2011` on 195; 21 primary meshes contain both.
  DEM10A is all `jgd2000`; DEM5/DEM1 in the backup are all `jgd2011`. The
  label is what the JGD2024 correction must look at — use `labels` /
  `served/dem10-labels.tsv`, not the primary mesh.
- **`gdalbuildvrt` skips inputs silently.** See rule 1. It cost the served
  stack 69 DEM10B grids (now in the `-fill` COGs). Never discard its stderr.
- **Overlapping products were painted in arbitrary order.** All 96 DEM10A
  meshes also have DEM10B; in 42 of them the served COG shows DEM10B (the
  coarser contour model) instead of DEM10A. DEM5 overlaps are common
  (9,376 tertiary meshes have 5A and 5B, 7,386 have 5A and 5C, 319 have 5B
  and 5C) and are likewise arbitrary in the served dem5 COGs except 4730 and
  5139. A rebuild with this tool applies 10B < 10A and 5C < 5B < 5A; the
  served files have not been replaced.
- **Tohoku: `jgd2000`-labelled DEM10B holds pre-2011 heights.** In the area
  of GSI's 2011 Tohoku-oki height revision (Aomori to Ibaraki,
  `touhokutaiheiyouoki2011_h.par`) the `jgd2000` DEM10B grids are not just
  labelled JGD2000, their heights *are* the pre-earthquake ones: the
  2016-10-01 edition equals the 2009-02-01 edition pixel for pixel (checked on
  7 meshes, e.g. Oshika 574134). 890 meshes are affected, 512 of them by more
  than 10 cm; at Oshika DEM10B is about 1.2 m above JGD2011. Outside that
  area the `jgd2000` label carries JGD2011-equivalent values (e.g. Hokkaido
  654506 changed label only), with possible residuals up to ~0.2 m in
  Kanto/Niigata/Nagano. Consequences: the current JGD2011 stack mixes DEM5
  (2011) and DEM10 (2000) heights there, and a JGD2024 correction of those
  grids needs `H + dH2011 + ΔH2024`, not `ΔH2024` alone (not implemented;
  a policy decision).
- **Furen-ko is a flat 0 m surface.** In `6545-fill.tif` (DEM10B
  654501/654502) one connected area of 125,282 pixels is exactly 0.0 m
  (tuple kind その他): the northern part of the Furen-ko lagoon, 43.333–43.358 N,
  145.23–145.31 E, cut off by the primary-mesh edge (the lagoon continues
  into 6445). It is data, not NoData — a water surface at 0 m. Do not treat
  0 m as missing.
- **GSI re-issued DEM1A/5A/5B/5C as JGD2024** on 2025-07-31 (editions dated
  2025-04 onwards, `srsName="fguuid:jgd2024.bl"`); DEM10A/B are not
  re-issued. Fetching "latest" DEM5 today therefore yields JGD2024 grids;
  `gml2tif` refuses them, use `fetch --before 2025-04-01` (or `--as-listed`)
  for this stack. The JGD2024 dataset is a separate stack.
- **The backup is a 2025-01-29 snapshot.** `base/dem5/4730.tif` and
  `5139.tif` were since rebuilt from newer (still JGD2011) editions; every
  other base COG comes from the backup.

## Publishing

**Anything put under `base/`, `patch/` or `sea/` goes live within minutes**:
the Worker lists those prefixes on every `config.json` request
(`cloudflare/tiles/src/r2.ts` `tileConfig`, `max-age=60`), and the tile
server revalidates that config about every 60 s (`CONFIG_TTL_SECS`) into
`plateau-terrain-experimental`. There is no staging. `publish` therefore:

1. is a **dry run unless `--execute`** (the dry run reads the bucket listing
   and `config.json` only);
2. accepts only keys of the form `base/dem<NN>/<mesh4>[-fill].tif`,
   `patch/<group>/<name>.tif`, `sea/<name>.tif` (the Worker's prefixes, pinned
   to r2.ts by a test), each with its manifest, which goes to
   `manifests/<key>.manifest.json`; absolute keys and `..` are refused;
3. requires the manifest to name the key and match the file's size and sha256;
4. **never overwrites**: every key is looked up by listing its parent
   directory (`rclone lsjson --stat` reports a missing S3 object as a
   directory with exit 0), any existing key aborts — also in a dry run — and
   uploads use `rclone copyto --immutable`. Replacing a served COG is a
   manual operation outside this tool;
5. verifies each object over the public URL (HEAD `content-length`, sha256
   of the body);
6. polls `config.json` (cache-busted) until it lists every uploaded COG and
   nothing else changed, and every `-fill` is painted under its main COG;
   then polls the tile server's `sources.json` (`sources` is a list of
   `{"name", "layers"}` objects) until `plateau-terrain-experimental` lists
   them (`--pickup-timeout`, default 420 s).

Any failure after the first upload — a failed check, a timeout, an
exception, Ctrl-C — deletes exactly the objects this run uploaded (exit 3/4/5);
a failed delete exits 6 and names what is left. All HTTP requests send a
browser User-Agent (Cloudflare answers 403 to Python's default).

## Ported from the scratch scripts, and what was not

Ported: the base DEM conversion of `convert-cogs.sh` (`convert_base`,
`buildvrt_checked`, `split_by_epsg`, majority → main / minority → fill,
refuse > 2 CRSs) with explicit product order; the patch conversion
(`_patch_region`, NoData/CRS uniformity, geo vs 3857 recipes) with the source
paths in `stacks.toml`; the gap-fill builders (FG-GML parsing, one COG per
primary and CRS, VRT source-count check); the strict per-pixel check of the
served stack; GSI catalogue/download; the manual upload plans, as the
guarded `publish`.

Not ported:

- **Ortho COGs** (`convert_ortho`, `plateau-ortho`) — not DEM; out of scope.
- **`provision-vm.sh` and the Spot-VM resume logic** (skip outputs that
  already exist in the bucket) — infrastructure for a nationwide batch;
  `build` is idempotent locally (`--force` to rebuild) and never writes to
  the bucket, so there is nothing to resume there.
- **The FME stages of the backup** (`s1_…fmw` FG-GML → GeoTIFF is replaced
  by `gml2tif`; the `s2-*`/`s3-*` ellipsoidal-height and terrain-tiler stages
  belong to the quantized-mesh mirror, not to this stack).
- **`sea/`** — the Worker enumerates it but nothing is there and no builder
  exists.
- dem1 secondary-mesh splitting (an open TODO of the batch README; dem1 is
  still one COG per primary).

## Files

| file | role |
|---|---|
| `demcog.py` | entry point (subcommands above) |
| `stacks.toml`, `stacks.py` | stacks (products, paint order, spacing, key prefix), patch sources, defaults |
| `meshcode.py` | mesh codes, grid file names, datum labels |
| `fggml.py` | FG-GML XML → per-grid GeoTIFF |
| `fetch.py` | GSI catalogue, edition choice, download, `gml2tif` provenance |
| `sources.py` | source trees (local / backup `/vsis3/`), `list-backup`, `mirror`, header scans |
| `inventory.py` | datum label of every backup grid |
| `plan.py` | CRS split, paint order, guards (pure) |
| `build.py` | COG build + manifest |
| `patch.py` | patch COG build |
| `verify.py` | `strict`, `reproduce` |
| `publish.py` | guarded upload |
| `served/` | the served state (see above) |
| `test_*.py` | `uv run demcog.py test` |
