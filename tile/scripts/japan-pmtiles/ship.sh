#!/usr/bin/env bash
# ship.sh — mirror Mapterhorn's Japan PMTiles and optionally ship it to the cloud.
#
# Usage:
#   ./ship.sh <target> [flags]
#
# Targets:
#   download      Mirror locally; keep the archive in $OUT_DIR (default ./out).
#                 No upload.
#   r2            Mirror to a temp dir, upload to Cloudflare R2, then delete
#                 the temp copy.
#   s3            Same for AWS S3 (or any S3-compatible service via $S3_ENDPOINT).
#   gcs           Same for Google Cloud Storage.
#
# Flags:
#   --keep        For r2/s3/gcs: also write a persistent copy into $OUT_DIR.
#   --dry-run     Estimate download size only (no fetch, no upload).
#
# Environment — mirror:
#   OUT_DIR         Destination dir for kept copies (default: ./out)
#   PMTILES_NAME    Output filename (default: japan-terrain.pmtiles)
#   WORKDIR         Intermediate work dir (default: ./work)
#   MIN_ZOOM        (default: 0)
#   MAX_ZOOM        (default: 14)
#   DOWNLOAD_BASE   (default: https://download.mapterhorn.com)
#
# Environment — upload (varies by target):
#   R2_BUCKET R2_ACCOUNT_ID R2_ACCESS_KEY_ID R2_SECRET_ACCESS_KEY
#   R2_KEY (optional, default: basename)  R2_PUBLIC_HOST (optional)
#   R2_CACHE_CONTROL (optional)
#
#   S3_BUCKET S3_REGION S3_KEY S3_ENDPOINT S3_ACL S3_CACHE_CONTROL
#   (AWS creds via env or ~/.aws/credentials)
#
#   GCS_BUCKET GCS_KEY GCS_CACHE_CONTROL
#   (auth via `gcloud auth application-default login` or
#   GOOGLE_APPLICATION_CREDENTIALS)
#
# Requirements: bash, curl, jq, awk, pmtiles CLI.
# Uploading needs: aws (for r2/s3) or gcloud/gsutil (for gcs).
#
# Why a temp dir and not a true pipe? `pmtiles merge` writes to a file with
# `os.Create` (needs random access to rewrite the header with final tile
# offsets); PMTiles v3 is not streamable by design. We fake no-persistent-
# local-file by uploading from a temp path and `rm -rf`-ing it.
set -euo pipefail

cd "$(dirname "$0")"

TARGET="${1:-}"
shift || true
KEEP_LOCAL=0

# Parse flags.
for arg in "$@"; do
    case "$arg" in
        --keep)     KEEP_LOCAL=1 ;;
        --dry-run)  export DRY_RUN=1 ;;
        *)
            echo "Unknown flag: $arg" >&2
            exit 2
            ;;
    esac
done

case "$TARGET" in
    download|r2|s3|gcs) ;;
    ""|--help|-h)
        sed -n '2,/^set -euo/p' "$0" | sed 's/^# \?//' | head -n -2
        exit 0
        ;;
    *)
        echo "Unknown target: $TARGET (expected: download | r2 | s3 | gcs)" >&2
        exit 2
        ;;
esac

OUT_DIR="${OUT_DIR:-$(pwd)/out}"
PMTILES_NAME="${PMTILES_NAME:-japan-terrain.pmtiles}"
WORKDIR="${WORKDIR:-$(pwd)/work}"
MIN_ZOOM="${MIN_ZOOM:-0}"
MAX_ZOOM="${MAX_ZOOM:-14}"
DOWNLOAD_BASE="${DOWNLOAD_BASE:-https://download.mapterhorn.com}"
DRY_RUN="${DRY_RUN:-0}"

# Japan bbox.
JP_WEST=122.0
JP_SOUTH=20.0
JP_EAST=154.0
JP_NORTH=46.0

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing tool: $1" >&2; exit 1; }; }
need curl; need jq; need awk; need pmtiles

# Where the final merged archive lands. For download mode: OUT_DIR (kept).
# For upload modes: a temp dir that we delete unless --keep was passed.
if [[ "$TARGET" == "download" ]]; then
    FINAL_DIR="$OUT_DIR"
    cleanup() { :; }
elif [[ "$DRY_RUN" == "1" || "$DRY_RUN" == "true" ]]; then
    FINAL_DIR="$WORKDIR"
    cleanup() { :; }
else
    FINAL_DIR="$(mktemp -d -t japan-pmtiles-XXXXXX)"
    cleanup() { rm -rf "$FINAL_DIR"; }
    trap cleanup EXIT
fi
mkdir -p "$FINAL_DIR" "$WORKDIR"
OUTPUT_PMTILES="$FINAL_DIR/$PMTILES_NAME"

echo "==> Target:   $TARGET"
echo "==> Output:   $OUTPUT_PMTILES"
echo "==> Zoom:     $MIN_ZOOM..$MAX_ZOOM  bbox=[$JP_WEST,$JP_SOUTH,$JP_EAST,$JP_NORTH]"
[[ "$DRY_RUN" == "1" || "$DRY_RUN" == "true" ]] && echo "==> DRY RUN (no files written, no upload)"

# ── Mirror ──────────────────────────────────────────────────────────────
INDEX_JSON="$WORKDIR/download_urls.json"
echo "==> Fetching Mapterhorn PMTiles index"
curl -fsSL "$DOWNLOAD_BASE/download_urls.json" -o "$INDEX_JSON"

SELECTED_JSON="$WORKDIR/selected.json"
jq --argjson w "$JP_WEST" --argjson s "$JP_SOUTH" \
   --argjson e "$JP_EAST" --argjson n "$JP_NORTH" \
   --argjson zmin "$MIN_ZOOM" --argjson zmax "$MAX_ZOOM" \
   '[.items[]
     | select(
         .min_lon < $e and .max_lon > $w and
         .min_lat < $n and .max_lat > $s and
         .min_zoom <= $zmax and .max_zoom >= $zmin
       )]' "$INDEX_JSON" > "$SELECTED_JSON"

NUM=$(jq 'length' "$SELECTED_JSON")
TOTAL_SIZE=$(jq '[.[] | .size] | add // 0' "$SELECTED_JSON")
echo "==> $NUM archives match (upstream total: $(( TOTAL_SIZE / 1024 / 1024 )) MB; bbox slice is much smaller)"

EXTRACTS_DIR="$WORKDIR/extracts"
LOG_DIR="$WORKDIR/logs"
mkdir -p "$EXTRACTS_DIR" "$LOG_DIR"

i=0
TOTAL_BYTES=0
while IFS=$'\t' read -r name url min_zoom max_zoom; do
    i=$((i+1))
    out="$EXTRACTS_DIR/$name"
    if [[ -f "$out" && "$DRY_RUN" != "1" && "$DRY_RUN" != "true" ]]; then
        echo "    [$i/$NUM] cached: $name"
        continue
    fi
    effective_min=$(( min_zoom > MIN_ZOOM ? min_zoom : MIN_ZOOM ))
    effective_max=$(( max_zoom < MAX_ZOOM ? max_zoom : MAX_ZOOM ))

    args=(extract
        --bbox="$JP_WEST,$JP_SOUTH,$JP_EAST,$JP_NORTH"
        --minzoom="$effective_min"
        --maxzoom="$effective_max"
    )
    if [[ "$DRY_RUN" == "1" || "$DRY_RUN" == "true" ]]; then
        args+=(--dry-run)
        echo "    [$i/$NUM] dry-run: $name (z=$effective_min..$effective_max)"
    else
        echo "    [$i/$NUM] extracting $name (z=$effective_min..$effective_max)"
    fi
    args+=("$url" "$out")

    log="$LOG_DIR/$name.log"
    pmtiles "${args[@]}" 2>&1 | tee "$log"

    bytes=$(awk '
        /total transfer|estimated|download size|bytes|would download/ {
            for (i=1;i<=NF;i++) if ($i ~ /^[0-9]+$/) last=$i
        }
        END { print last+0 }
    ' "$log")
    TOTAL_BYTES=$(( TOTAL_BYTES + bytes ))
done < <(jq -r '.[] | [.name, .url, .min_zoom, .max_zoom] | @tsv' "$SELECTED_JSON")

if [[ "$DRY_RUN" == "1" || "$DRY_RUN" == "true" ]]; then
    echo
    echo "==> DRY-RUN SUMMARY"
    echo "    archives:            $NUM"
    if [[ "$TOTAL_BYTES" -gt 0 ]]; then
        printf "    estimated download:  %'d bytes (~%.2f GB)\n" \
            "$TOTAL_BYTES" "$(awk -v b=$TOTAL_BYTES 'BEGIN{print b/1024/1024/1024}')"
    fi
    echo "    (no files were written; drop --dry-run to run for real)"
    exit 0
fi

echo "==> Merging $NUM extracts → $OUTPUT_PMTILES"
pmtiles merge "$OUTPUT_PMTILES" "$EXTRACTS_DIR"/*.pmtiles
ls -lh "$OUTPUT_PMTILES"

# ── Upload ──────────────────────────────────────────────────────────────
upload_s3_compatible() {
    # $1 endpoint (empty for AWS), $2 bucket, $3 key, $4 region,
    # $5 acl (optional), $6 cache-control (optional)
    local endpoint="$1" bucket="$2" key="$3" region="$4" acl="$5" cc="$6"
    need aws
    local args=(s3 cp "$OUTPUT_PMTILES" "s3://${bucket}/${key}"
                --content-type application/octet-stream)
    [[ -n "$endpoint" ]] && args=(--endpoint-url "$endpoint" "${args[@]}")
    [[ -n "$acl" ]]      && args+=(--acl "$acl")
    [[ -n "$cc" ]]       && args+=(--cache-control "$cc")
    AWS_DEFAULT_REGION="${region:-${AWS_DEFAULT_REGION:-}}" aws "${args[@]}"
}

case "$TARGET" in
    download)
        echo "==> Done: $OUTPUT_PMTILES (kept locally)"
        ;;
    r2)
        : "${R2_BUCKET:?R2_BUCKET required}"
        : "${R2_ACCOUNT_ID:?R2_ACCOUNT_ID required}"
        : "${R2_ACCESS_KEY_ID:?R2_ACCESS_KEY_ID required}"
        : "${R2_SECRET_ACCESS_KEY:?R2_SECRET_ACCESS_KEY required}"
        key="${R2_KEY:-$PMTILES_NAME}"
        endpoint="https://${R2_ACCOUNT_ID}.r2.cloudflarestorage.com"

        echo "==> Uploading to r2://${R2_BUCKET}/${key}"
        AWS_ACCESS_KEY_ID="$R2_ACCESS_KEY_ID" \
        AWS_SECRET_ACCESS_KEY="$R2_SECRET_ACCESS_KEY" \
        upload_s3_compatible "$endpoint" "$R2_BUCKET" "$key" "auto" "" \
            "${R2_CACHE_CONTROL:-}"
        [[ -n "${R2_PUBLIC_HOST:-}" ]] && \
            echo "    Public URL: https://${R2_PUBLIC_HOST}/${key}"
        ;;
    s3)
        : "${S3_BUCKET:?S3_BUCKET required}"
        key="${S3_KEY:-$PMTILES_NAME}"
        echo "==> Uploading to s3://${S3_BUCKET}/${key}"
        upload_s3_compatible \
            "${S3_ENDPOINT:-}" "$S3_BUCKET" "$key" \
            "${S3_REGION:-${AWS_DEFAULT_REGION:-}}" \
            "${S3_ACL:-}" "${S3_CACHE_CONTROL:-}"
        if [[ -n "${S3_ENDPOINT:-}" ]]; then
            echo "    Endpoint: $S3_ENDPOINT"
        elif [[ -n "${S3_REGION:-}" ]]; then
            echo "    Public URL (if public): https://${S3_BUCKET}.s3.${S3_REGION}.amazonaws.com/${key}"
        fi
        ;;
    gcs)
        : "${GCS_BUCKET:?GCS_BUCKET required}"
        key="${GCS_KEY:-$PMTILES_NAME}"
        echo "==> Uploading to gs://${GCS_BUCKET}/${key}"
        if command -v gcloud >/dev/null 2>&1; then
            gcs_args=(storage cp "$OUTPUT_PMTILES" "gs://${GCS_BUCKET}/${key}")
            [[ -n "${GCS_CACHE_CONTROL:-}" ]] && gcs_args+=(--cache-control="$GCS_CACHE_CONTROL")
            gcloud "${gcs_args[@]}"
        elif command -v gsutil >/dev/null 2>&1; then
            headers=()
            [[ -n "${GCS_CACHE_CONTROL:-}" ]] && headers+=(-h "Cache-Control:$GCS_CACHE_CONTROL")
            gsutil "${headers[@]}" cp "$OUTPUT_PMTILES" "gs://${GCS_BUCKET}/${key}"
        else
            echo "gcloud or gsutil required." >&2
            exit 1
        fi
        echo "    Public URL (if bucket is public):"
        echo "    https://storage.googleapis.com/${GCS_BUCKET}/${key}"
        ;;
esac

# Optionally persist a local copy too.
if [[ "$TARGET" != "download" && "$KEEP_LOCAL" == "1" ]]; then
    mkdir -p "$OUT_DIR"
    cp "$OUTPUT_PMTILES" "$OUT_DIR/"
    echo "==> Kept local copy: $OUT_DIR/$PMTILES_NAME"
fi

echo "==> Done."
