#!/usr/bin/env bash
# Smoke test of a deployed PLATEAU Server: checks that the main public endpoints answer
# with real data. Read-only requests only.
#
# Usage: .github/scripts/smoke-test-server.sh [base_url]
#   base_url defaults to https://api.plateauview.mlit.go.jp
set -euo pipefail

BASE_URL="${1:-https://api.plateauview.mlit.go.jp}"
BASE_URL="${BASE_URL%/}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

failed=0

# A new Cloud Run revision may need a moment to start, so retry each request.
fetch() {
  curl -sS --fail-with-body --max-time 60 --retry 5 --retry-delay 5 --retry-all-errors "$@"
}

check() {
  local name="$1"
  shift
  if "$@"; then
    echo "ok   $name"
  else
    echo "FAIL $name"
    failed=1
  fi
}

ping_ok() {
  [ "$(fetch "$BASE_URL/ping")" = '"pong"' ]
}

datasets_ok() {
  fetch "$BASE_URL/datacatalog/plateau-datasets" -o "$TMP/datasets.json" &&
    jq -e '.datasets | length > 1000' "$TMP/datasets.json" >/dev/null
}

graphql_ok() {
  fetch -X POST "$BASE_URL/datacatalog/graphql" \
    -H 'content-type: application/json' \
    -d '{"query":"{ datasets(input: {areaCodes: [\"13101\"], includeTypes: [\"bldg\"]}) { id items { id url } } }"}' \
    -o "$TMP/graphql.json" &&
    jq -e '(.errors | not) and (.data.datasets | length > 0) and (.data.datasets[0].items | length > 0)' "$TMP/graphql.json" >/dev/null
}

composite_tileset_ok() {
  fetch "$BASE_URL/datacatalog/3dtiles/13101-bldg-lod2-latest/tileset.json" -o "$TMP/tileset.json" &&
    jq -e '.root.children | length > 0' "$TMP/tileset.json" >/dev/null
}

echo "Smoke testing $BASE_URL"
check "GET /ping" ping_ok
check "GET /datacatalog/plateau-datasets" datasets_ok
check "POST /datacatalog/graphql" graphql_ok
check "GET /datacatalog/3dtiles/13101-bldg-lod2-latest/tileset.json" composite_tileset_ok

exit "$failed"
