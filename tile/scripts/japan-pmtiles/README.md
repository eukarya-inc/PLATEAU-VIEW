# japan-pmtiles — ship.sh

One script that mirrors Mapterhorn's Japan terrain PMTiles and optionally
ships it to R2 / S3 / GCS.

## How it works

Mapterhorn publishes PMTiles archives on
[`download.mapterhorn.com`](https://www.mapterhorn.com/data-access):

- `planet.pmtiles` — z0-12, globally (~706 GB)
- `{z6_x}-{z6_y}.pmtiles` — z13+, per-region

`pmtiles extract --bbox=122,20,154,46` streams only the bytes that fall
inside the bbox. `ship.sh` applies that to every archive that intersects
Japan and `pmtiles merge`s the results into a single file. For upload
targets the file lives in a `mktemp` directory and is deleted after upload.

No image processing. No re-encoding. PMTiles v3 can't be streamed directly
to stdout (the format writes offset tables at the start of the file and
needs random access), so there's always a short-lived local merged file —
but in upload mode it never lands in your working directory.

## Install

```bash
brew install pmtiles jq           # macOS
# or: go install github.com/protomaps/go-pmtiles@latest

brew install awscli               # for R2 / S3 uploads
brew install --cask gcloud-cli    # for GCS uploads
```

## Run

```bash
# Mirror locally (kept in ./out/)
./ship.sh download

# Mirror + upload + auto-cleanup
./ship.sh r2
./ship.sh s3
./ship.sh gcs

# Mirror + upload + also keep a local copy in ./out/
./ship.sh r2 --keep

# Estimate download size without touching anything
./ship.sh download --dry-run
./ship.sh r2       --dry-run
```

### Mirror knobs

| Var | Default | |
|---|---|---|
| `OUT_DIR` | `./out` | Destination for `download` / `--keep` copies |
| `PMTILES_NAME` | `japan-terrain.pmtiles` | Filename |
| `WORKDIR` | `./work` | Cached per-archive extracts (safe to delete) |
| `MIN_ZOOM` | `0` | |
| `MAX_ZOOM` | `14` | |
| `DOWNLOAD_BASE` | `https://download.mapterhorn.com` | |
| `DRY_RUN` | — | Set `1` instead of passing `--dry-run` |

### Upload credentials

```bash
# Cloudflare R2
export R2_BUCKET=my-terrain
export R2_ACCOUNT_ID=xxxxxxxx
export R2_ACCESS_KEY_ID=xxxx
export R2_SECRET_ACCESS_KEY=xxxx
export R2_PUBLIC_HOST=pub-xxxxxxxx.r2.dev  # optional, prints final URL
export R2_KEY=japan-dem-v1.pmtiles          # optional, default is basename
export R2_CACHE_CONTROL="public, max-age=3600"  # optional

# AWS S3 (or any S3-compatible service via S3_ENDPOINT)
export S3_BUCKET=my-terrain
export S3_REGION=ap-northeast-1
export S3_ACL=public-read                       # optional
export S3_CACHE_CONTROL="public, max-age=3600"  # optional
# AWS creds via env vars or ~/.aws/credentials.

# Google Cloud Storage
gcloud auth application-default login
export GCS_BUCKET=my-terrain
export GCS_KEY=japan-dem-v1.pmtiles
export GCS_CACHE_CONTROL="public, max-age=3600"  # optional
```

## Point the tile server at it

```jsonc
{
  "terrain": {
    "dem": {
      "type": "pmtiles",
      "url": "https://pub-xxxxxxxx.r2.dev/japan-dem-v1.pmtiles",
      "encoding": "terrarium",
      "version": "v1",
      "maxZoom": 14,
      "nativeTileSize": 512
    }
  }
}
```

Mapterhorn tiles are 512 px WebP Terrarium — carried through as-is.

Restart the tile server. The PMTiles archive's HTTP ETag is automatically
mixed into downstream cache keys, so **updating the archive in place
transparently invalidates all serving caches** without a CDN partial purge.

## Refresh

Re-run `./ship.sh r2` (or your target). Either rely on upstream ETag
propagation, or bump `dem.version` in the tile config for an explicit break.
