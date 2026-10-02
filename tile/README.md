# tile

High-performance tile server with Cloud Optimized GeoTIFF (COG) overlay support, written in Rust.

## Features

- **XYZ Tile Proxy**: Fetch and serve tiles from remote XYZ tile servers
- **COG Tile Generation**: Generate tiles from Cloud Optimized GeoTIFF files with HTTP range requests
- **Layer Composition**: Overlay multiple COG layers on top of base XYZ tiles
- **Multi-band NoData**: Support for multi-band nodata values with multiple patterns (e.g., black AND white as transparent)
- **Auto Overview Selection**: Automatically select the best resolution overview for each zoom level
- **Bilinear Interpolation**: Smooth tile rendering with bilinear interpolation between COG pixel centres (a tile pixel that coincides with a source pixel returns it exactly; the outer half of a COG's edge pixels is clamped, so there is no fringe along COG borders)
- **Memory Caching**: Fast in-memory tile cache using moka
- **Remote Configuration**: Load configuration from remote URL with manual reload
- **HTTP/2 (h2c)**: Support for HTTP/2 cleartext connections (auto-detects HTTP/1.1 and HTTP/2)
- **Smart ETag**: Per-tile ETag calculation based on covering layers with If-None-Match support for 304 responses
- **Configurable Cache-Control**: Set custom Cache-Control headers via environment variable
- **Multi-Format Output**: Support for PNG, WebP, and AVIF image formats
- **Terrain**: Cesium quantized-mesh-1.0 (TMS Geodetic) plus Mapzen Terrarium and Mapbox Terrain-RGB raster tiles (Web Mercator XYZ), generated from a Mapterhorn DEM and a per-source `japan-geoid` model (GSIGEO2011 / JPGEO2024 / JPGEO2024+Hrefconv). Heights are **ellipsoidal** (orthometric + geoid) by default; `?heights=` also serves the orthometric DEM or the geoid surface alone. Tiles fully outside the geoid coverage respond 404.

## Terrain

The server generates Cesium quantized-mesh-1.0 (`/terrain/`, **TMS Geodetic** addressing as Cesium expects) plus Mapzen Terrarium (`/terrarium/`) and Mapbox Terrain-RGB v1 (`/mapbox/`) raster tiles on the fly from a DEM source plus a geoid model. The two raster endpoints serve **Web Mercator XYZ** tiles — same projection as a normal MapLibre/Mapbox raster source. The output is in **ellipsoidal heights**, ready to drop into Cesium (or any MapLibre style that consumes Mapbox/Terrarium DEM) without a vertical-datum mismatch against 3D Tiles or geocoded data.

### Why Mapterhorn for the DEM

[Mapterhorn](https://www.mapterhorn.com) is our default DEM upstream because of how it's packaged and licensed:

- **Single source, global coverage with high-resolution Japan.** Mapterhorn merges many open national DEMs into one consistent set. For Japan it builds on **国土地理院 (GSI)** elevation data, so we get the local resolution we need without stitching multiple providers ourselves.
- **Distributed as PMTiles.** Every region is a single `.pmtiles` archive, and `pmtiles extract --bbox=...` downloads only the bytes inside a bounding box. That makes a Japan-only, production-ready mirror a single command — see [`scripts/japan-pmtiles/`](scripts/japan-pmtiles/).
- **Friendly licensing.** Code is BSD-3, terrain data is CC-BY-4.0 / OGL / CC0 family. The full attribution list is at [mapterhorn.com/attribution](https://www.mapterhorn.com/attribution); every DEM source built on the Mapterhorn base credits Mapterhorn and 国土地理院 in its `layer.json` / TileJSONs automatically (see [Attribution](#attribution)).
- **Modern format.** 512 px Terrarium-encoded WebP — smaller transfers and a strict drop-in replacement for the deprecated AWS Elevation Tiles.

In production the recommended setup is to mirror Mapterhorn's Japan slice into your own R2 / GCS bucket and point the tile server at it; this avoids hitting `tiles.mapterhorn.com` for every request and keeps you in control of cache invalidation.

### Why a separate geoid model

Mapterhorn (and most public DEM tile services) encode **orthometric** heights — height above the geoid, i.e. roughly mean sea level. Cesium's globe is the WGS84 ellipsoid, so feeding orthometric heights directly causes a 30–40 m vertical offset over Japan, which makes 3D Tiles buildings float or sink. We resolve this by adding a **geoid height** at every grid point, using the [`japan-geoid`](https://crates.io/crates/japan-geoid) crate (GSIGEO2011 / JPGEO2024 / JPGEO2024+Hrefconv).

#### The model belongs to the data, the mode to the request

A geoid model is bound to a **vertical datum**: GSIGEO2011 goes with JGD2011 orthometric heights, JPGEO2024 with JGD2024. Combining a DEM with a geoid from the other datum produces plausible-looking numbers that mean nothing. So the model is **declared per DEM source** in the config JSON (`"geoid": "..."`), falling back to `TERRAIN_DEFAULT_GEOID`, and **cannot be chosen by a request**. Models are never mixed and never fall back to one another.

What a request *can* choose is the vertical surface, via `?heights=`:

| `heights=` | Surface | Notes |
|------------|---------|-------|
| `ellipsoidal` | orthometric DEM + geoid | **Default** — what an unparameterised request returns |
| `orthometric` | the DEM as-is | No geoid applied |
| `geoid` | the geoid surface alone | DEM elevation ignored; useful for inspecting the model a source is bound to |

Aliases: `ortho`, `ellipsoid`, `geoid-only`. Each mode lives in its own cache key and ETag, so the three can be served side by side without a partial purge.

> **Breaking change.** The old `?geoid=<model>` selector is **gone**. Any request that still carries a `geoid` query parameter gets a **400** naming `heights=` and its valid values — it is never silently ignored and never silently falls back to another model. `?geoid=none` in particular becomes `?heights=orthometric`.

Tiles whose bounds lie **entirely outside** the source's geoid coverage box respond `404` — in every height mode, since coverage is a property of the source's model.

#### Where the model has no value: the 0 fill and `X-Geoid-Coverage`

Inside that box the model's grid still has **no value** over most of the sea, over foreign land (e.g. the Korean peninsula for GSIGEO2011) and at some remote islands. Those samples are currently served with a geoid height of **0**: `ellipsoidal` output there is just the orthometric height — tens of metres off the true ellipsoidal height — and `geoid` reads 0 m. This is an interim policy, not a datum statement. Whether and how to extrapolate a single model beyond its grid is pending consultation with MLIT; models are never mixed to fill the gap.

So that clients can tell where the fill was used, DEM-generated tile responses (`/terrain/{source}/…`, `/terrarium/…`, `/mapbox/…`) carry an informational header:

| `X-Geoid-Coverage` | Meaning |
|---|---|
| `full` | The model had a value at every sample of the tile |
| `partial` | Some samples used the 0 fill |
| `none` | Every sample used the 0 fill: `ellipsoidal` heights there equal the orthometric DEM, and `heights=geoid` is a flat 0 m surface |

- "Samples" are the 65×65 grid of a quantized-mesh tile (the normal-computation halo excluded) or the pixel centres of a raster tile. The value depends only on the model and those positions, not on the DEM.
- Sent for `heights=ellipsoidal` and `heights=geoid`. **Omitted** for `heights=orthometric` (no geoid is involved), on 404s, on `304 Not Modified`, and on the quantized-mesh mirror backend (pre-rendered tiles, no geoid at request time).
- The header never changes a response body, status code, ETag or cache key. Tiles served from the memory or persistent cache return the same value: it is remembered from the render and, when that memory is gone, recomputed from the same sample positions.
- With an explicit `CORS_ORIGINS` list the header is listed in `Access-Control-Expose-Headers`; with `*` every header is exposed.

### Layering DEM overlays on top of the base

The base DEM is set via `DEM_URL` (env var). To **patch in higher-resolution data over a specific area** — for example a city-level COG, a regional Terrarium PMTiles, or an XYZ DEM service — declare a special source named **`dem`** in the config JSON:

```jsonc
{
  "sources": {
    "ortho": { "layers": [/* regular raster layers */] },

    "dem": {
      // Geoid model this source's elevations are referenced to:
      // "gsigeo2011" (default) | "jpgeo2024" | "jpgeo2024-hrefconv".
      // Omit to inherit TERRAIN_DEFAULT_GEOID.
      "geoid": "gsigeo2011",
      "layers": [
        // Order matters: index 0 = bottom-most overlay, last = frontmost.
        { "type": "pmtiles", "url": "gs://my-bucket/japan-2m.pmtiles",
          "encoding": "terrarium", "version": "v1",
          "maxZoom": 14, "nativeTileSize": 512 },

        { "type": "cog", "url": "https://.../tokyo-1m.tif",
          "version": "tokyo-2025q1", "nodata": -9999 },

        { "type": "xyz", "url": "https://.../detailed/{z}/{x}/{y}.png",
          "encoding": "mapbox", "maxZoom": 18, "nativeTileSize": 256 }
      ]
    }
  }
}
```

- The source named `"dem"` is **not** exposed under `/tiles/dem/...`; its layers feed the terrain endpoint instead.
- `"geoid"` fixes the vertical datum for that source. Two DEM sources can therefore carry different models — e.g. a JGD2011 `dem` alongside a JGD2024 `dem-2024` (which may reuse the JGD2011 COGs through a [height correction](#vertical-datums-and-the-jgd2011--jgd2024-height-correction)) — each addressable at `/terrain/{name}/...`. An unrecognised value is logged at ERROR and the source falls back to `TERRAIN_DEFAULT_GEOID`; it never silently picks a neighbouring model.
- Each overlay paints over the layers below it pixel-by-pixel. Where an overlay has no data (NaN / nodata), the layer underneath shows through.
- Where the base DEM has no tile at the requested zoom (Mapterhorn 404s offshore above roughly z6), the composite bilinear-upsamples the nearest parent. Child pixels within half a parent pixel of the parent's border clamp to the edge sample, so the result is NaN-free wherever the parent is.
- At startup, every COG / PMTiles overlay's metadata is fetched in parallel and indexed into an R*-tree. Per-tile rendering only fetches overlays whose bbox intersects the tile, so the cost stays flat as you add more local overlays.
- Cache keys aggregate base + every overlay's ETag (or `failed:slug` markers when an overlay's fetch fails for that tile), so updating any archive in place rolls all serving caches without a CDN partial purge.
- Each pod refreshes every COG overlay's upstream ETag every **5 minutes** (single HEAD per overlay), so a CMS-side file swap that doesn't bump the config hash is picked up automatically — no `/reload` needed. The pod's own memory and persistent caches invalidate on ETag mismatch; downstream HTTP caches still honour their `Cache-Control: max-age` so end-users see the new tiles after at most one CDN TTL.

### Vertical datums and the JGD2011 → JGD2024 height correction

A geoid model only gives correct ellipsoidal heights when every elevation it is added to is in *its* orthometric datum. GSI's 2025 標高改定 moved Japanese heights from JGD2011 to JGD2024 by a few centimetres to ~0.4 m, so a JGD2024 source built from JGD2011 COGs needs that change — ΔH — added to each elevation. The server applies it **on the fly, per layer**; the COGs are never rewritten.

```jsonc
"plateau-terrain-jgd2024": {
  "type": "dem",
  "geoid": "jpgeo2024-hrefconv",   // target datum: jgd2024
  "verticalDatum": "jgd2011",      // default for every layer below
  "base": "sealevel",              // flat 0 m base instead of the shared DEM_URL base
  "heightCorrection": {
    "manifest": "https://tiles.plateau.city/terrain/vertical/hyokorev-jgd2011-to-jgd2024/v1/manifest.json",
    "missing": "keep"              // keep | nan — see below (pending a decision)
  },
  "layers": [
    { "type": "cog", "url": ".../base/dem5/5339.tif" },
    { "type": "cog", "url": ".../jgd2024/dem5/5339.tif", "verticalDatum": "jgd2024" }  // per-layer override
  ]
}
```

- **Target datum** = the datum of the source's `geoid`: `gsigeo2011` → `jgd2011`, `jpgeo2024-hrefconv` → `jgd2024`. `jpgeo2024` alone has no target datum (without Hrefconv2024 it is off from JGD2024 heights by up to ~0.7 m on remote islands), so a source using it cannot declare datums or a correction.
- **Layer datum** = the layer's `verticalDatum`, else the source's. Once a source declares any datum, every layer must resolve to one. Mixed stacks are the intended next step: when GSI's JGD2024 DEM5A/DEM1A re-issue replaces those layers, they are declared `jgd2024` (served as-is) while the remaining JGD2011 layers (e.g. DEM10) keep getting ΔH.
- **Base** = the source's `"base"`: omitted → the shared `DEM_URL` base (what every existing source uses, unchanged); `"sealevel"` → the flat 0 m sea-level base for this source only. Its slug is part of the source's slug, version and per-tile etag, so switching it re-keys that source only. The intended JGD2024 setup uses `"sealevel"`, so no DEM of another datum (or another edition) shows through its gaps — they are 0 m = mean sea level instead.
- **Base datum** = for the shared base, `DEM_VERTICAL_DATUM` (`jgd2011` | `jgd2024`) for a real `DEM_URL` upstream, **unknown** when unset. The sea-level base (`"base": "sealevel"` or `DEM_URL=sealevel`) is **datum-agnostic** and is **never** corrected: it means "no data → 0 m = mean sea level", and adding ΔH to it would warp the sea surface. A source on the sea-level base therefore needs no `DEM_VERTICAL_DATUM`.
- Every member (base or layer) whose datum differs from the target gets ΔH added to each elevation **before** the priority composite, at the exact points that member's elevations were evaluated (a geographic COG's XYZ pixels are latitude-linear within the tile, a Mercator upstream's are Mercator pixel centres; parent-tile upsampling is followed through). The composite is therefore in one datum, and the geoid is added once at the end as before. The quantized-mesh 65×65 grid and its normal halo are warped from that corrected composite, so they carry the same correction.
- **`heightCorrection.manifest`** names the correction product by the URL of its `manifest.json` (the version is in the path, so swapping versions is a config change). The manifest's files are fetched once per process through the object-store backends (https / gs / s3 / r2 / file), checked against the manifest's size and sha256, decoded from the COGs' full-resolution image only, and kept in memory (only the non-empty 512² blocks: 30 MiB for v1, vs. ~49 MiB dense). ΔH is evaluated exactly like the reference sampler `scripts/vertical-cogs/sampler.py` — including GSI's selection rule over `dh_bm.tif` / `dh_tr.tif` / `tr_meshes.json` and the v1 manifest layout (no `selection` block) — and a golden test pins the Rust sampler to vectors generated by that reference (bit-identical on all 2,510 finite vectors). The sampler lives in `crates/terrain-core`; the same vectors are re-checked there over `fixtures/vertical/*.golden-nodes.json` (only the grid nodes the points read, regenerated with `cargo test -p tile --lib -- --ignored bless_golden_nodes`), natively and in wasm32.
- **`heightCorrection.missing`** — what to do with an elevation that needs ΔH where the product has none (Northern Territories, Iwo-to, the Senkaku islands, Nanatsu-jima, open sea): `keep` serves it uncorrected, `nan` drops the pixel so lower layers (ultimately the base) show through. **This is a pending policy decision**; `keep` is the default only because it changes nothing that is served today. The choice is part of the cache key.
- **Refused at config load** (logged at ERROR; the source is not served — a refused `dem` does *not* fall back to the bare base): a layer or base in a datum other than the target with no `heightCorrection`; a correction whose product does not convert exactly *layer datum → target* (e.g. a `jgd2024` layer in a `jgd2011` source — no inverse correction exists); a correction on a base of unknown datum; a geoid without a target datum; unknown `verticalDatum` / `missing` / `base` values; a product that fails to load or verify.
- **Known limitation — Tohoku 2011 height revision (pending an MLIT decision).** In the area GSI revised after the 2011 Tohoku earthquake (Aomori, Iwate, Miyagi, Akita, Yamagata, Fukushima, Ibaraki; parameter `touhokutaiheiyouoki2011_h.par`), the `base/dem10` COGs are labelled JGD2000 (EPSG:4612) and hold **pre-2011 (測地成果2000) heights**. They are bit-identical to GSI's 2009-02-01 DEM10B editions and up to ~+1.2 m above JGD2011 near Oshika. The JGD2024 correction assumes JGD2011 input, so those `dem10` pixels come out under-corrected there by the 2011 revision. Chaining the 2011 correction before ΔH is not implemented; whether to do it is pending a decision. The datum model would express it as a third datum on those layers.
- **Cache keys / ETags**: a source that applies a correction appends `vcorr=<product>@<version>:<from>-to-<to>:missing=<policy>:base=<datum>:base-dh=<yes|no>:layers=<mask hash>` to its DEM version, which feeds every terrain cache key and ETag. A source that declares nothing — every existing source — is built exactly as before and keeps its keys byte for byte (no cache flush).

### Attribution

Every DEM source's `layer.json` and terrain TileJSONs (`/terrain/{source}/layer.json`, `/terrarium/{source}/tilejson.json`, `/mapbox/{source}/tilejson.json`) carry an `attribution` **derived from what that source is built from** (`src/terrain/attribution.rs`), so the credit cannot drift from the config. Parts, joined with ` | ` (exact duplicates dropped):

| Part | Credited when |
|------|---------------|
| PLATEAU | always |
| Base | the source uses the shared `DEM_URL` base: `DEM_ATTRIBUTION` if set, else Mapterhorn (the `{z}/{x}/{y}` template default and the PMTiles mirror from `scripts/japan-pmtiles/`). Nothing for the sea-level base (`"base": "sealevel"` or `DEM_URL=sealevel`) |
| Layers | each overlay layer's own `"attribution"`, bottom → top, when it declares one |
| 国土地理院 | always — every source applies a GSI geoid model, and overlay layers without their own `"attribution"` are taken to be GSI elevation data (as the PLATEAU COGs are). A source that applies a `heightCorrection` gets `国土地理院（基盤地図情報 数値標高モデル、標高補正パラメータを適用して加工）`, since GSI's terms ask processed data to say so (wording pending review) |

A source's own `"attribution"` (HTML) replaces the derived line entirely — for cases derivation cannot know, e.g. COGs whose provenance the server can't see:

```jsonc
"dem": { "type": "dem", "attribution": "<a href=\"https://www.mlit.go.jp/plateau/\">PLATEAU</a> | …", "layers": [ … ] }
```

What that gives the sources served today (`DEM_URL` = Mapterhorn):

| Source | `attribution` |
|--------|---------------|
| `dem`, `plateau-terrain-experimental` (shared base + COGs) | `PLATEAU \| Mapterhorn \| 国土地理院` (unchanged) |
| `plateau-terrain-jgd2024` (`"base": "sealevel"` + `heightCorrection`) | `PLATEAU \| 国土地理院（基盤地図情報 数値標高モデル、標高補正パラメータを適用して加工）` |
| `/terrain/layer.json` from the quantized-mesh mirror | `PLATEAU \| Mapterhorn \| 国土地理院` (fixed; the mirror has no config source) |

The attribution is metadata only: it never enters a tile body, an ETag or a cache key, so changing it needs no cache flush. `/tiles/{name}/tilejson.json` honours the same source-level `"attribution"` field and otherwise keeps the PLATEAU credit.

### Supported COG tile compressions

COG tiles are decoded via `async_tiff`, whose default decoders cover **uncompressed, Deflate, LZW, JPEG, and ZSTD**, plus **WebP** — GDAL's private TIFF compression tag `50001` — via the crate's `webp` feature, which this crate enables in `Cargo.toml`. So ortho-imagery COGs built with `-co COMPRESS=WEBP` decode correctly. (Up to async-tiff 0.2 the WebP decoder was a local one in `src/cog/webp.rs`; 0.3.0 ships an equivalent upstream, so the local copy is gone.) Recommended choices:

- **DEM (float32 elevation):** `ZSTD` (or `DEFLATE`) with `PREDICTOR=3` — lossless, see below.
- **Ortho (RGB imagery):** `WEBP` or `JPEG` — both are decodable and give a good size/quality trade-off for aerial photography.

### Preparing COG DEM overlays

When you build a COG to use as a `dem` overlay, **how you generate the overviews matters a lot at low zooms**. The default `gdal_translate -of COG` pipeline uses `average` resampling for overviews — which, even when nodata-aware, has a footprint-growing behavior: any 2×2 group with *at least one* valid pixel keeps a valid value in the parent. After five levels (×32 downsample) a single 5 m land pixel in the middle of the sea has spread to a ~160 m square block of "land" surrounding it.

When `CompositeDemProvider` then paints that bloated land block over the base DEM (which correctly reads ~0 m for the surrounding sea), you get **tall thin "spikes" around every small island, harbour, and coastline** at low zooms. The footprint mismatch is the problem, not the elevation values themselves.

The fix is to **build overviews with `gdaladdo -r nearest`** (no footprint growth — small features either survive at their exact position or get dropped, both of which are visually fine) and then **tell the COG driver to use those existing overviews** instead of regenerating with `average`. If you're starting from multiple source tiles, also **mosaic them with `gdalwarp -r near`** — anything else (default `bilinear`, `cubic`, …) blends real elevations with the nodata sentinel at every mask boundary and seeds the same spike pattern at the *source* level, which then leaks into every downstream overview:

```bash
# 0. (Only if you have multiple source tiles.) Mosaic to a single GeoTIFF
#    with nearest-neighbour resampling so nodata never blends with real
#    elevations. Set -srcnodata and -dstnodata explicitly to the sentinel.
gdalwarp -r near -multi \
  -co COMPRESS=ZSTD -co PREDICTOR=3 -co TILED=YES -co BIGTIFF=IF_SAFER \
  -srcnodata <SENTINEL> -dstnodata <SENTINEL> \
  -t_srs EPSG:4326 \
  source-tiles/*.tif input.tif

# 1. Translate to a plain tiled GeoTIFF (no overviews yet). If you ran
#    step 0 with the COG-compatible compression options above, you can
#    skip this step and let gdaladdo write overviews into `input.tif`
#    directly.
gdal_translate -of GTiff \
  -co COMPRESS=ZSTD -co PREDICTOR=3 -co TILED=YES -co BIGTIFF=IF_SAFER \
  input.tif tmp.tif

# 2. Build footprint-preserving overviews with nearest-neighbour.
gdaladdo -r nearest tmp.tif 2 4 8 16 32

# 3. Wrap as COG, reusing the overviews we just built.
gdal_translate -of COG \
  -co COMPRESS=ZSTD -co PREDICTOR=3 \
  -co OVERVIEWS=FORCE_USE_EXISTING -co BIGTIFF=IF_SAFER \
  tmp.tif output.tif

rm tmp.tif
```

Make sure the band's `nodata` value is set on the input before step 2 (`gdal_edit.py -a_nodata <value> input.tif` if it isn't). `gdaladdo` reads it from the band metadata, and `nearest` won't synthesise spurious values that fall outside your real elevation range.

#### Picking a nodata sentinel

Any sentinel works for the tile server, but **small magnitudes are easier on every other tool** in your pipeline. Recommended (in order of preference):

1. **`-9999`** — the de-facto DEM convention. Tiny tolerance gap, never collides with real Japanese elevations (lowest land < −10 m, highest peak < 4000 m), survives 16-bit signed integer formats if you ever downcast.
2. **`255` / `0`** — fine for tightly-bounded datasets (e.g. an urban DSM that can't physically be near those values).
3. **`f32::MIN` (`-3.4028235e+38`)** — avoid if you can. `gdalwarp` accepts it but the value is so extreme that *any* non-nearest resampler creates fringe values across many orders of magnitude (we've seen `-2.7e+37` in production), which then have to be caught by the server's defensive guards rather than the strict nodata check. Stick to `-9999` and the data stays clean for QGIS, downstream processors, and statistics tools too.

> 🛡️ **Defense-in-depth in the server.** Even if a corrupted COG slips through, the tile server has three independent guards so a single bad pixel can no longer black out an entire terrain tile (the failure mode we observed in production with a `f32::MIN`-sentinel overlay):
>
> 1. **Adaptive nodata tolerance** (`src/cog/reader.rs`) — `max(0.5 m, |nodata| · 1e-3)`. Small sentinels get the 0.5 m floor that catches `254.99996` next to `255`; huge sentinels get a proportional band wide enough to absorb bilinear-blended fringe values.
> 2. **Physical elevation guard** (`src/cog/decode.rs`, `MAX_PHYSICAL_ELEVATION_M = 50_000`) — anything beyond ±50 km is dropped to NaN at decode time. Mt. Everest is 8.85 km, Mariana Trench −10.9 km; anything bigger is corruption.
> 3. **Output sanitisation** (`sanitize_height` in `src/terrain/mesh_gen.rs`) — every served height that is non-finite or beyond ±50 km becomes 0 m. The Martini sample callback uses it so a stray bad value can't drag `min_height` to −10³⁷ and collapse the quantized-mesh bounding sphere or horizon-occlusion point, and the `/terrarium` / `/mapbox` encoders use it so the same sample reads 0 m there too instead of the format floor (−32768 m / −10000 m).
>
> Mosaicking with `-r near` is still the right thing to do — keeping the data clean upstream means *other* tools (QGIS, downstream processors) also see well-formed values.

> ⚠️ The COG driver-only option for adopting pre-built overviews is **`OVERVIEWS=FORCE_USE_EXISTING`**. The GTiff-driver equivalent `COPY_SRC_OVERVIEWS=YES` is silently ignored by `-of COG` (GDAL only emits a `Warning 6`), and the result is a COG whose overviews were regenerated with `average` — exactly the spike-producing case.

> ℹ️ **Don't reach for `-r average` or `-r mode` to "smooth" things.** Both also keep a pixel valid whenever the 2×2 group contains *any* valid pixel, so they grow the land footprint identically (measured +6 pp valid-pixel ratio at OV4 vs. +0.2 pp for `nearest`). `mode` preserves peak elevations slightly better, but produces the same spike pattern. They are only safe when your dataset has no nodata mask at all.

To re-check after build:

```bash
# Should print OVERVIEW_RESAMPLING=NEAREST and the per-overview NoData/Min/Max.
gdalinfo -mm output.tif | grep -E "(OVERVIEW_RESAMPLING|NoData|Overviews|Min/Max)"

# Sanity-check that the valid-pixel ratio is *stable* across overview levels
# (a level that "grows" the land area is the symptom of averaging spillover):
python3 -c "
from osgeo import gdal
ds = gdal.Open('output.tif'); b = ds.GetRasterBand(1); nd = b.GetNoDataValue()
for i in range(b.GetOverviewCount()):
    a = b.GetOverview(i).ReadAsArray()
    print(f'OV{i}: {100*(a!=nd).sum()/a.size:.1f}% valid')
"
```

## Quick Start

### Build

```bash
cargo build --release
```

### Run

```bash
CONFIG_URL=https://example.com/config.json ./target/release/tile
```

### Docker

```bash
docker build -t tile .
docker run -e CONFIG_URL=https://example.com/config.json -p 8080:8080 tile
```

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `CONFIG_URL` | No | - | Config JSON URL(s). Omit to run with terrain-only defaults; required only to enable `/tiles/...` sources. Accepts a **comma-separated list** — each URL is fetched and their `sources` merged in list order (earlier wins on a name collision), so independent authorities can each publish a config |
| `CONFIG_TTL_SECS` | No | `60` | Lazy revalidation TTL. Each tile/terrain request checks whether the config has been re-fetched within this window; on the first miss per pod the request synchronously re-fetches and, if the body hash changed, rebuilds sources before serving. `0` disables (manual `/reload` only). Synchronous on purpose — Cloud Run throttles CPU outside the active request, so a background poller could be paused or killed mid-rebuild |
| `PORT` | No | `8080` | HTTP server port |
| `CACHE_SIZE_MB` | No | `512` | Memory cache size in MB |
| `RELOAD_SECRET` | No | - | Secret token for `/reload` endpoint (if set, requires `Authorization: Bearer <token>`) |
| `CORS_ORIGINS` | No | `*` | Allowed CORS origins (comma-separated, or `*` for all) |
| `PRELOAD_MODE` | No | `sync` | COG metadata preload mode: `sync` (blocking), `background` (non-blocking), or `lazy` (on first request) |
| `TILE_CACHE_URL` | No | - | Persistent tile cache URL (see below) |
| `CACHE_CONTROL` | No | `public, max-age=3600, must-revalidate` | Cache-Control header value for tile responses |
| `NO_CACHE` | No | - | If truthy (`1`/`true`/`yes`/`on`), disables all caching: memory cache → 0, persistent cache → off, `Cache-Control` → `no-store, must-revalidate`. Handy during local terrain iteration |
| `RUST_LOG` | No | `info` | Log level (trace, debug, info, warn, error) |

### Terrain Variables

The terrain endpoint's base DEM and output settings are operational concerns and live in env vars (the config JSON only describes `/tiles/...` overlay sources).

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DEM_URL` | No | Mapterhorn public endpoint | DEM source URL. If it ends with `.pmtiles`, the server reads it as a PMTiles archive. Schemes: `https://` (any HTTPS host), `gs://bucket/key` (Google Cloud Storage, supports private via ADC / `GOOGLE_APPLICATION_CREDENTIALS`), `s3://bucket/key` (AWS S3 / S3-compatible), `r2://bucket/key` (Cloudflare R2 — set `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`), `file:///path/to.pmtiles`. The sentinel value `sealevel` (aliases: `sea-level`, `none`, `0m`) selects a flat 0 m base instead of a real upstream — terrain then comes solely from the `dem` source's overlays, and uncovered pixels render at the geoid height (mean sea level). Anything else is treated as a Mapterhorn-style `{z}/{x}/{y}` template |
| `DEM_VERSION` | No | `v1` | Internal version key, mixed into cache keys. Bump for an explicit cache break |
| `DEM_MAX_ZOOM` | No | `15` | Upstream DEM max zoom (clamps `/terrain/` requests above this) |
| `DEM_NATIVE_TILE_SIZE` | No | `512` | Native tile pixel size in the upstream archive (PMTiles only; Mapterhorn is always 512) |
| `TERRAIN_TILE_SIZE` | No | `256` | Output raster tile pixel size for `/terrarium/` and `/mapbox/`. A DEM tile of another native size is resampled between pixel centres — for Mapterhorn's 512 px tiles at the default 256, each output pixel is the mean of a 2×2 block |
| `TERRAIN_DEFAULT_GEOID` | No | `gsigeo2011` | Geoid model for DEM sources that don't declare their own `geoid` in the config JSON. One of `gsigeo2011`, `jpgeo2024`, `jpgeo2024-hrefconv`. **Not** overridable per request |
| `DEM_ATTRIBUTION` | No | Mapterhorn credit | Credit (HTML) for the shared `DEM_URL` base, part of the derived `attribution` of every DEM source built on it (see [Attribution](#attribution)). Set it when `DEM_URL` points at something other than Mapterhorn or a Mapterhorn mirror. Ignored for `DEM_URL=sealevel` |
| `DEM_VERTICAL_DATUM` | No | — (unknown) | Vertical datum of the shared `DEM_URL` base: `jgd2011` or `jgd2024`. Only read by sources that declare vertical datums and use the shared base (see [Vertical datums](#vertical-datums-and-the-jgd2011--jgd2024-height-correction)); such a source with a `heightCorrection` refuses to build while it is unset. Not needed for sources with `"base": "sealevel"`; ignored for `DEM_URL=sealevel`, which is datum-agnostic and never corrected |
| `TERRAIN_MAX_ZOOM` | No | `18` | Max zoom advertised in `/terrain/layer.json` and the raster `tilejson.json` endpoints. Above `DEM_MAX_ZOOM` both the quantized-mesh and raster endpoints fall back to the parent DEM tile and bilinear-upsample the relevant sub-region, sampling at child pixel centres. |
| `TERRAIN_MAX_ERROR` | No | `5.0` | Martini mesh-simplification error in meters (lower = more triangles) |
| `TERRAIN_MIRROR_URL` | No | — | Pre-rendered quantized-mesh mirror bucket (`r2://`, `s3://`, `gs://`, `file://`). When set, `/terrain/`, `/terrain/mirror/`, and `/terrain-mirror/` serve directly from this bucket instead of generating from DEM. The DEM pipeline remains reachable at `/terrain/dem/` for side-by-side validation. See [Quantized-mesh mirror](#quantized-mesh-mirror-pre-rendered-passthrough) below. |

```bash
# Self-hosted PMTiles on a public R2 bucket (HTTPS)
DEM_URL=https://pub-xxxx.r2.dev/japan-dem-v1.pmtiles cargo run

# Private GCS bucket (uses application-default credentials)
DEM_URL=gs://my-private-bucket/japan-dem-v1.pmtiles cargo run

# Private R2 bucket (S3-compatible)
R2_ACCOUNT_ID=xxxx R2_ACCESS_KEY_ID=xxxx R2_SECRET_ACCESS_KEY=xxxx \
  DEM_URL=r2://my-bucket/japan-dem-v1.pmtiles cargo run

# Local file
DEM_URL=file:///abs/path/to/japan-dem-v1.pmtiles cargo run

# Flat 0 m base (no upstream DEM) — terrain comes solely from config-JSON overlays
DEM_URL=sealevel cargo run

# Default Mapterhorn upstream — fine for development
cargo run
```

### Quantized-mesh mirror (pre-rendered pass-through)

`/terrain/` has a second backend: a pass-through mirror that serves pre-rendered quantized-mesh tiles read straight from an object-store bucket — no DEM, no Martini, no geoid composition at request time. Use this when a sibling job (e.g. [`eukarya-inc/ion-terrain-mirror`](https://github.com/eukarya-inc/ion-terrain-mirror)) has already populated a bucket with `{prefix}/layer.json` + `{prefix}/{z}/{x}/{y}.terrain` (gzipped) under the [Cesium Ion mirror layout](https://github.com/eukarya-inc/ion-terrain-mirror).

```bash
# R2 (S3-compatible): mirror takes over /terrain/
R2_ACCOUNT_ID=xxx R2_ACCESS_KEY_ID=xxx R2_SECRET_ACCESS_KEY=xxx \
  TERRAIN_MIRROR_URL=r2://plateau-terrain/plateau-terrain-2024/ \
  cargo run

# Local files (smoke testing)
TERRAIN_MIRROR_URL=file:///abs/path/to/mirror/ cargo run
```

Routing once `TERRAIN_MIRROR_URL` is set:

| URL                                        | Backend |
|--------------------------------------------|---------|
| `/terrain/layer.json`                      | Mirror  |
| `/terrain/{z}/{x}/{y}.terrain`             | Mirror  |
| `/terrain/mirror/{z}/{x}/{y}.terrain`      | Mirror  |
| `/terrain-mirror/{z}/{x}/{y}.terrain`      | Mirror (separate route — always the mirror; 404 when `TERRAIN_MIRROR_URL` is unset) |
| `/terrain/dem/{z}/{x}/{y}.terrain`         | DEM pipeline (Mapterhorn etc.) — used for side-by-side validation |
| `/terrain/{other}/...`                     | DEM, looked up under the source name in the config JSON |

The mirror handler stamps `Content-Encoding: gzip` and the standard quantized-mesh `Content-Type` on every tile response and rewrites `attribution` + `tiles` on the upstream `layer.json` so the credit line and tile template match the DEM pipeline over the Mapterhorn base (fixed `PLATEAU | Mapterhorn | 国土地理院` — the mirror has no config source to derive one from; relative `{z}/{x}/{y}.terrain`). `/terrarium/...` and `/mapbox/...` raster endpoints are unaffected — they still go through the DEM pipeline.

### Persistent Cache Configuration

The tile server supports two-tier caching: fast in-memory cache (moka) + optional persistent storage.

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `TILE_CACHE_URL` | No | - | Persistent cache URL (`file://`, `gs://`, `s3://`, `r2://`) |
| `TILE_CACHE_MODE` | No | `read-write` | Cache mode: `read-write`, `read-only`, `write-only`, or `none` |
| `TILE_CACHE_CONTROL` | No | - | Cache-Control header for objects stored in persistent cache (e.g., `public, max-age=31536000`) |
| `R2_ACCOUNT_ID` | For R2 | - | Cloudflare R2 account ID |
| `R2_ACCESS_KEY_ID` | For R2 | - | Cloudflare R2 access key ID |
| `R2_SECRET_ACCESS_KEY` | For R2 | - | Cloudflare R2 secret access key |

#### Cache URL Examples

```bash
# Local file cache
TILE_CACHE_URL=file:///var/cache/tiles

# Google Cloud Storage
TILE_CACHE_URL=gs://my-bucket/tiles

# Amazon S3
TILE_CACHE_URL=s3://my-bucket/tiles

# Cloudflare R2 (requires R2_* env vars)
TILE_CACHE_URL=r2://my-bucket/tiles
```

#### Cache Modes

| Mode | Read Persistent | Write Persistent | Use Case |
|------|-----------------|------------------|----------|
| `read-write` | Yes | Yes | Default. Server is primary cache source |
| `read-only` | Yes | No | Persistent storage is managed externally |
| `write-only` | No | Yes | CDN/Worker handles reads (e.g., Cloudflare Worker + R2) |
| `none` | No | No | Disable persistent cache entirely |

All modes always use the in-memory cache (moka). Persistent failures don't block tile serving (fail-safe).

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/tiles/:name/tilejson.json` | Get TileJSON metadata for a source |
| GET | `/tiles/:name/:z/:x/:y.{format}` | Get a tile (format: `png`, `webp`, `avif`) |
| GET | `/terrain/layer.json` | Cesium quantized-mesh-1.0 layer.json. Query: `?heights=ellipsoidal\|orthometric\|geoid` (default `ellipsoidal`) |
| GET | `/terrain/:z/:x/:y.terrain` | Quantized-mesh-1.0 tile (gzipped, octvertexnormals). Same `?heights=` query |
| GET | `/terrarium/tilejson.json` | TileJSON for the Terrarium raster output. Query: `?heights=...&format=png\|webp\|avif` |
| GET | `/terrarium/:z/:x/:y.{format}` | Terrarium raster of **ellipsoidal** heights by default (orthometric DEM + geoid offset); `?heights=` selects the surface |
| GET | `/mapbox/tilejson.json` | TileJSON for the Mapbox Terrain-RGB v1 raster output |
| GET | `/mapbox/:z/:x/:y.{format}` | Mapbox Terrain-RGB v1 raster of ellipsoidal heights |
| GET | `/terrain-viewer` | Embedded Cesium preview of the terrain output |
| GET | `/health` | Health check |
| POST | `/reload` | Force-reload configuration (requires `Authorization: Bearer <RELOAD_SECRET>` if secret is set). Always rebuilds sources, even when the config body hash is unchanged. Most operators don't need to call this — see `CONFIG_TTL_SECS` for the lazy-revalidation path that picks up CMS changes automatically |

> See [Terrain](#terrain) above for what these endpoints output and how the `heights` query parameter works. To self-host the DEM, see [`scripts/japan-pmtiles/`](scripts/japan-pmtiles/).

### Supported Image Formats

| Extension | MIME Type | Description |
|-----------|-----------|-------------|
| `.png` | `image/png` | Lossless compression, best for graphics with transparency |
| `.webp` | `image/webp` | Modern format with good compression and transparency support |
| `.avif` | `image/avif` | Best compression ratio, newer format with growing browser support |

Example requests:
```bash
# PNG (default, widest compatibility)
curl https://example.com/tiles/ortho/10/909/403.png

# WebP (smaller file size, good browser support)
curl https://example.com/tiles/ortho/10/909/403.webp

# AVIF (smallest file size, modern browsers)
curl https://example.com/tiles/ortho/10/909/403.avif
```

### TileJSON

Each source provides a TileJSON 3.0.0 endpoint for integration with mapping libraries:

```bash
# Default format (PNG)
curl https://example.com/tiles/ortho/tilejson.json

# Specify format
curl https://example.com/tiles/ortho/tilejson.json?format=webp
```

Response:
```json
{
  "tilejson": "3.0.0",
  "tiles": ["https://example.com/tiles/ortho/{z}/{x}/{y}.png"],
  "name": "ortho",
  "attribution": "<a href=\"https://www.mlit.go.jp/plateau/\" target=\"_blank\">PLATEAU</a>",
  "scheme": "xyz",
  "minzoom": 0,
  "maxzoom": 22
}
```

`attribution` is the source's `"attribution"` from the config JSON when set, else the PLATEAU credit shown above.

Query parameters:
| Parameter | Default | Description |
|-----------|---------|-------------|
| `format` | `png` | Tile format: `png`, `webp`, or `avif` |

## Configuration

Configuration is loaded from a remote JSON file specified by `CONFIG_URL`.

`CONFIG_URL` may list several comma-separated URLs (`file://`, `http(s)://`).
Each is fetched and their `sources` are merged in list order; on a source-name
collision the **earlier** URL wins (list order = precedence) and the duplicate
is logged. The merged `version` joins each config's version with `+`, and the
combined content hash folds in every body, so `/reload` (and lazy revalidation)
picks up a change in any of them. This lets independent authorities each publish
a config — e.g. the CMS-derived config plus a Cloudflare Worker that enumerates
R2 COGs — without either having to know the other's contents:

```bash
CONFIG_URL="https://cms.example/config.json,https://tiles.example/r2-cogs.json" ./target/release/tile
```

### Example Configuration

```json
{
  "version": "v1.0.0",
  "sources": {
    "plateau-ortho": {
      "version": "v1.0.1",
      "layers": [
        {
          "type": "xyz",
          "url": "https://example.com/tiles/{z}/{x}/{y}.png",
          "range": {
            "z_min": 0,
            "z_max": 18
          }
        },
        {
          "type": "cog",
          "url": "https://storage.googleapis.com/bucket/ortho/area1.tif",
          "nodata": [[0, 0, 0], [255, 255, 255]],
          "order": 1
        },
        {
          "type": "cog",
          "url": "gs://bucket/ortho/area2.tif",
          "nodata": [[0, 0, 0]],
          "order": 2
        }
      ]
    },
    "dem": {
      // Geoid model this source's elevations are referenced to:
      // "gsigeo2011" (default) | "jpgeo2024" | "jpgeo2024-hrefconv".
      // Omit to inherit TERRAIN_DEFAULT_GEOID.
      "geoid": "gsigeo2011",
      "layers": [
        {
          "type": "cog",
          "url": "https://storage.googleapis.com/bucket/dem/elevation.tif"
        }
      ]
    }
  }
}
```

### Layer Types

#### XYZ Layer

Fetches tiles from a remote XYZ tile server.

```json
{
  "type": "xyz",
  "url": "https://example.com/{z}/{x}/{y}.png",
  "range": {
    "z_min": 0,
    "z_max": 18,
    "x_min": 0,
    "x_max": 1000,
    "y_min": 0,
    "y_max": 1000
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | URL template with `{z}`, `{x}`, `{y}` placeholders |
| `range` | object | No | Coordinate range restriction |

#### COG Layer

Generates tiles from a Cloud Optimized GeoTIFF file.

Supported CRS (from the GeoTIFF EPSG key):

- **Geographic degrees** — WGS84 (EPSG:4326) and JGD2011 / JGD2000 geographic (EPSG:6668 / 4612). The latter are treated as WGS84; the datum shift over Japan is sub-pixel, so the error is negligible. A COG with no EPSG key is assumed to be WGS84.
- **Web Mercator meters** — EPSG:3857 (and the legacy `3785` alias). Since XYZ tiles *are* Web Mercator, requests map to a perfect square in this space and sampling is exact (no latitude distortion). Bounds are reprojected to WGS84 only for catalog/intersection bookkeeping.

Any other CRS (e.g. a JGD2011 *plane rectangular* system in meters, or UTM) is rejected at open time.

```json
{
  "type": "cog",
  "url": "https://storage.googleapis.com/bucket/image.tif",
  "nodata": [[0, 0, 0], [255, 255, 255]],
  "order": 1
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | URL to COG file (HTTP, `gs://`, `s3://`, `r2://`) |
| `nodata` | various | No | NoData value configuration (see below) |
| `order` | number | No | Layer order (higher = on top, default: 0) |

#### PMTiles Layer

Reads image tiles from a PMTiles archive. Same URL schemes as DEM PMTiles
(`https://`, `gs://`, `s3://`, `r2://`, `file://`).

```json
{
  "type": "pmtiles",
  "url": "https://pub-xxx.r2.dev/imagery.pmtiles",
  "range": { "z_min": 0, "z_max": 18 }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | URL to the `.pmtiles` archive |
| `range` | object | No | Coordinate range restriction |

When this layer type is used inside the special `sources.dem` source, the
extra DEM-only fields `encoding` (`terrarium` \| `mapbox`), `maxZoom`, and
`nativeTileSize` apply.

### NoData Configuration

NoData values can be specified in multiple formats:

```json
// Single value (all bands must match)
"nodata": 255

// Single pattern (RGB black)
"nodata": [0, 0, 0]

// Multiple patterns (black OR white)
"nodata": [[0, 0, 0], [255, 255, 255]]
```

Pixels matching any nodata pattern will be rendered as transparent.

### ETag and Cache Control

The tile server supports HTTP caching through ETag headers and configurable Cache-Control.

#### Smart ETag Calculation

ETags are calculated per-tile based on which layers actually cover that specific tile. This enables granular cache invalidation:

- **Per-tile calculation**: Each tile's ETag only includes layers that cover it
- **COG bounds awareness**: Tiles outside a COG's bounds won't be invalidated when the COG changes
- **Version support**: Each layer can have an optional `version` for cache invalidation

```json
{
  "sources": {
    "ortho": {
      "layers": [
        {
          "type": "xyz",
          "url": "https://example.com/{z}/{x}/{y}.png",
          "version": "v1.0.0"
        },
        {
          "type": "cog",
          "url": "https://example.com/overlay.tif",
          "version": "v2.0.0",
          "order": 1
        }
      ]
    }
  }
}
```

| Field | Description |
|-------|-------------|
| `version` (layer) | Optional version string for cache invalidation. When changed, invalidates cache for tiles covered by this layer |

ETag calculation:
- `W/"xxhash64(source/format/z/x/y|key1|key2|...)"` where keys are from covering layers
- Each layer contributes a key like `type:url:version` (e.g., `xyz:https://example.com/{z}/{x}/{y}.png:v1.0.0`)
- Format is included in ETag, so different formats have different ETags
- Clients can send `If-None-Match` header to receive `304 Not Modified` if cache is valid
- **Granular invalidation**: Changing a COG layer's version only invalidates tiles within that COG's geographic bounds
- COG layers also carry the server's COG sampling version (`cog::COG_SAMPLING_VERSION`, currently `centre-v1`), so a change in how COG pixels are sampled re-keys exactly the tiles COGs contribute to — for `/tiles` COG layers and, via the per-tile overlay ETag, for COG DEM overlays on `/terrain`, `/terrarium` and `/mapbox`. XYZ / PMTiles / MapLibre layers, Mapterhorn-only terrain tiles and the quantized-mesh mirror keep their keys.

#### Cache-Control Header

The `CACHE_CONTROL` environment variable controls HTTP caching behavior:

```bash
# Default (1 hour with must-revalidate for quick cache invalidation)
CACHE_CONTROL="public, max-age=3600, must-revalidate"

# CDN-friendly caching with longer edge cache
CACHE_CONTROL="public, max-age=3600, s-maxage=86400"

# No caching
CACHE_CONTROL="no-cache, no-store"
```

The default `must-revalidate` ensures that expired cache entries are always revalidated with the server, enabling quick propagation of cache invalidation.

### Supported COG URL Schemes

| Scheme | Example | Description |
|--------|---------|-------------|
| `http://`, `https://` | `https://example.com/file.tif` | HTTP/HTTPS with range request support |
| `gs://` | `gs://bucket/path/file.tif` | Google Cloud Storage |
| `s3://` | `s3://bucket/path/file.tif` | Amazon S3 |
| `r2://` | `r2://bucket/path/file.tif` | Cloudflare R2 (set `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`) |
| `file://` | `file:///abs/path/file.tif` | Local file — **DEM overlay layers (`type: "dem"` sources) only**, for tests and local validation. Raster `/tiles` COG layers do not accept it |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    HTTP Request                             │
│                GET /tiles/ortho/10/909/403.png              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Memory Cache (moka)                       │
└─────────────────────────────────────────────────────────────┘
                              │ miss
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Persistent Cache (optional)                    │
│            (file:// / gs:// / s3:// / r2://)               │
└─────────────────────────────────────────────────────────────┘
                              │ miss
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  CompositeTileSource                        │
├─────────────────────────────────────────────────────────────┤
│  1. XyzTileSource (base)                                    │
│     └─ Fetch from remote XYZ server                         │
│                                                             │
│  2. CogTileSource (overlay, order=1)                        │
│     ├─ Check intersection with COG bounds                   │
│     ├─ Select best IFD (overview) for zoom level            │
│     ├─ Fetch tiles via HTTP range requests                  │
│     ├─ Decode & apply nodata → transparent                  │
│     └─ Bilinear interpolation                               │
│                                                             │
│  3. CogTileSource (overlay, order=2)                        │
│     └─ Same as above                                        │
├─────────────────────────────────────────────────────────────┤
│                    Image Composition                        │
│              (alpha blending overlays)                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Image Response                            │
│            (PNG/WebP/AVIF, cached by format)                │
└─────────────────────────────────────────────────────────────┘
```

## Requirements

- COG files must be in a **supported CRS** — geographic degrees (WGS84 4326, JGD2011 6668, JGD2000 4612) or Web Mercator meters (3857, 3785). See [COG Layer](#cog-layer) for the full matrix.
- COG files should have internal tiling and overviews for best performance

### Creating COG Files

#### Choosing the CRS: it's really a resampling-pipeline decision

Picking the COG's CRS isn't a standalone choice — it's a question of **where you
resample and how to avoid resampling twice**. Every reprojection that changes the
pixel grid is one generation of quality loss, and **the per-tile resample from COG
buffer to the 256×256 output happens no matter what** (it's inherent to tile
serving). So the goal is: do any *grid-changing* reprojection **once, offline, at
build time** with a good kernel, and land the COG on the grid the data will
ultimately be consumed on.

For this server the final display grid is **Web Mercator**, which drives the rule:

| Source CRS | Recommended COG CRS | Why |
|------------|---------------------|-----|
| Already 3857 | **Keep 3857** | Zero grid-changing resamples at build; serve-time tile fit is an exact axis-aligned scale (no latitude distortion). |
| Already 4326 / 6668 | **Keep as-is** | Don't reproject to "normalize" — the server treats 6668 as 4326 (sub-pixel datum shift), so a warp would only add loss. Serve-time does the degree→tile mapping (a mild linear-in-latitude approximation, negligible at city zooms). |
| Some other projection (UTM, JGD2011 **plane rectangular** 6677…, …) | **Reproject straight to 3857** | You must reproject anyway (these aren't accepted). Going to 3857 collapses the build reproject *and* the inevitable serve resample into effectively one clean step — the second step degrades to same-grid scaling. Reprojecting to 4326 instead leaves two *different* grids (lat/lon then mercator) = two lossy passes. |

Rule of thumb: **serve-only asset → Web Mercator; data asset reused in GIS/analysis
→ WGS84** (Mercator distorts area/distance, ~1.22× scale at Japan's latitude).

#### Web Mercator: align to the XYZ grid for 1:1 overview selection

The server picks which overview (IFD) to read by resolution ratio
(`src/cog/reader.rs::select_best_ifd`): for a requested tile it computes
`required_res = tile_size / tile_extent` and reads the smallest overview whose
resolution clears `required_res × 1.5`. If the COG's overview levels line up with
the **standard XYZ zoom resolutions** — Web Mercator pixel size at zoom `z` is
`(2 × 20037508.34) / (2^z × tile_size)` — then each requested zoom maps to exactly
one overview and `resample_to_tile` copies buffer pixels **1:1** to the output:
sharpest result, least bytes fetched, lowest CPU.

The COG driver's `TILING_SCHEME` does all of this — reproject to EPSG:3857, align
the origin to tile-matrix boundaries, set the block size, and build a
zoom-aligned overview pyramid — in a single pass (think "gdal2tiles baked into a
COG"):

```bash
gdal_translate -of COG \
  -co TILING_SCHEME=GoogleMapsCompatible \
  -co BLOCKSIZE=256 \
  -co RESAMPLING=cubic \
  -co ZOOM_LEVEL_STRATEGY=AUTO \
  -co COMPRESS=DEFLATE \
  source.tif out_cog.tif
```

- `BLOCKSIZE` must match the server's `tile_size` (default **256**; use 512 only if you run the server at 512).
- `ZOOM_LEVEL_STRATEGY`: `LOWER` avoids upsampling the source, `UPPER` preserves all detail, `AUTO` rounds to the nearest zoom.
- `RESAMPLING`: `cubic`/`bilinear` for continuous imagery, `nearest`/`mode` for categorical rasters.
- **DEM overlays are the exception** — build overviews with `nearest` and mosaic with `-r near` to avoid nodata-blend spikes. See [Preparing COG DEM overlays](#preparing-cog-dem-overlays).

#### Geographic (WGS84 / JGD2011) — plain COG

When the source is already geographic, skip the reprojection and just wrap it as a
COG with internal tiling and overviews:

```bash
gdal_translate input.tif output_cog.tif \
  -of COG \
  -co COMPRESS=DEFLATE \
  -co OVERVIEW_RESAMPLING=BILINEAR
```

If the source is in some other CRS and you want a geographic (rather than Web
Mercator) COG, reproject **once** with a quality kernel — don't chain warps:

```bash
gdalwarp -t_srs EPSG:4326 -r cubic input.tif output_4326.tif
gdal_translate output_4326.tif output_cog.tif -of COG -co COMPRESS=DEFLATE
```

## Development

```bash
# Run with debug logging
RUST_LOG=debug CONFIG_URL=file:///path/to/config.json cargo run

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run linter
cargo clippy
```

`tile/` is a Cargo workspace: the server is the root package and
`crates/terrain-core` holds the pure, synchronous terrain primitives (DEM
sample positions and resampling, the ΔH sampler, the elevation sanitiser, the
overlay composite) that a WebAssembly build shares with the server. The
commands above cover both. To run the core's tests in wasm32 under Node
(the ΔH golden vectors must stay bit-identical there):

```bash
rustup target add wasm32-unknown-unknown
# wasm-bindgen-cli must match the wasm-bindgen version in Cargo.lock
cargo install wasm-bindgen-cli --version <that version> --locked
cargo test -p terrain-core --target wasm32-unknown-unknown --tests -- --nocapture
```

## License

Apache-2.0
