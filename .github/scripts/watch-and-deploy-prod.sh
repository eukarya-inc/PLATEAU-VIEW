#!/usr/bin/env bash
set -euo pipefail

# ===== 引数チェック =====
if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <target>"
  echo ""
  echo "Available targets:"
  echo "  server    # Watch and deploy PLATEAU Server"
  echo "  worker    # Watch and deploy PLATEAU Worker"
  echo "  tile      # Watch and deploy PLATEAU Tile"
  echo "  docs      # Watch and deploy PLATEAU Docs"
  echo ""
  echo "Examples:"
  echo "  $0 server"
  echo "  $0 tile"
  exit 1
fi

TARGET_TYPE="$1"

# ===== 設定 =====
CI_WORKFLOW_NAME="ci"                              # 最初に待機するCIワークフロー
TARGET_BRANCH="main"                               # CIを見るブランチ
POLL_INTERVAL=10                                   # run が見つかるまでのポーリング間隔（秒）
MAX_CHECKS=60                                      # 最大チェック回数

# ターゲットタイプに応じて設定を切り替え
case "$TARGET_TYPE" in
  server)
    DEV_WORKFLOW_FILE="deploy-server-dev.yml"
    DISPATCH_WORKFLOW_FILE="deploy-server-prod.yml"
    ;;
  worker)
    DEV_WORKFLOW_FILE="deploy-worker-dev.yml"
    DISPATCH_WORKFLOW_FILE="deploy-worker-prod.yml"
    ;;
  tile)
    DEV_WORKFLOW_FILE="deploy-tile-dev.yml"
    DISPATCH_WORKFLOW_FILE="deploy-tile-prod.yml"
    ;;
  docs)
    DEV_WORKFLOW_FILE="deploy-docs-dev.yml"
    DISPATCH_WORKFLOW_FILE="deploy-docs-prod.yml"
    ;;
  *)
    echo "Error: Invalid target type '$TARGET_TYPE'"
    echo "Must be one of: server, worker, tile, docs"
    exit 1
    ;;
esac

echo "Watching CI workflow '$CI_WORKFLOW_NAME' and Dev workflow '$DEV_WORKFLOW_FILE' on branch '$TARGET_BRANCH'..."
echo "On success, will dispatch '$DISPATCH_WORKFLOW_FILE'."
echo ""

# 最新コミットの情報を取得
LATEST_COMMIT_SHA=$(git rev-parse "$TARGET_BRANCH")
LATEST_COMMIT_DATE=$(git log -1 --format=%cI "$TARGET_BRANCH")
echo "Latest commit on $TARGET_BRANCH:"
echo "  SHA: $LATEST_COMMIT_SHA"
echo "  Date: $LATEST_COMMIT_DATE"
echo ""

# ===== ヘルパー関数 =====

# 指定ワークフローの最新コミットに対応する run ID を見つける
# 見つかったら run ID を出力して 0 を返す。見つからなければ 1 を返す。
find_run_for_commit() {
  local workflow="$1"
  local sha="$2"

  local run_json
  run_json=$(gh run list \
    --workflow "$workflow" \
    --branch "$TARGET_BRANCH" \
    -L 5 \
    --json databaseId,headSha,status,conclusion \
    --jq ".[] | select(.headSha == \"$sha\")" | head -1)

  if [[ -z "$run_json" || "$run_json" == "null" ]]; then
    return 1
  fi

  echo "$run_json" | jq -r '.databaseId'
}

# run が見つかるまでポーリングして待つ
wait_for_run() {
  local workflow="$1"
  local sha="$2"
  local label="$3"

  for i in $(seq 1 "$MAX_CHECKS"); do
    local run_id
    if run_id=$(find_run_for_commit "$workflow" "$sha"); then
      echo "$run_id"
      return 0
    fi
    echo "  [$i/$MAX_CHECKS] Waiting for $label run to appear for $sha..." >&2
    sleep "$POLL_INTERVAL"
  done

  echo "Timeout: $label run not found for $sha" >&2
  return 1
}

# ===== Step 1: CI ワークフローの完了を待つ =====
echo "Step 1: Waiting for CI workflow to complete..."
echo ""

CI_RUN_ID=$(wait_for_run "$CI_WORKFLOW_NAME" "$LATEST_COMMIT_SHA" "CI")
echo "Found CI run: $CI_RUN_ID"
gh run watch "$CI_RUN_ID" --exit-status
echo ""
echo "CI workflow succeeded."
echo ""

# ===== Step 2: Deploy Dev ワークフローの完了を待つ（存在する場合のみ） =====
echo "Step 2: Checking for Deploy Dev workflow..."
echo ""

if DEV_RUN_ID=$(find_run_for_commit "$DEV_WORKFLOW_FILE" "$LATEST_COMMIT_SHA"); then
  echo "Found Deploy Dev run: $DEV_RUN_ID"
  gh run watch "$DEV_RUN_ID" --exit-status
  echo ""
  echo "Deploy Dev workflow succeeded."
else
  echo "No Deploy Dev workflow found for this commit (dev deploy may be included in CI). Skipping."
fi
echo ""

# ===== Step 3: 本番デプロイのトリガーと監視 =====
# `workflow_dispatch` で起動された run の `headSha` は dispatch 時点の
# `$TARGET_BRANCH` の HEAD に解決される（= 我々が CI 通過させた SHA とは
# 限らない: 直後に他コミットが main に乗ると一致しなくなる）。
# よって SHA で照合せず、dispatch 直前のラン ID をベースラインに、
# それより新しいランが現れたら「我々が起こしたラン」とみなして直接追跡する。
echo "Step 3: Triggering production deployment..."
echo ""

# Dispatch 直前の最新ランをベースラインとして記録。
BASELINE_RUN_ID=$(gh run list \
  --workflow "$DISPATCH_WORKFLOW_FILE" \
  -L 1 \
  --json databaseId \
  --jq '.[0].databaseId // 0')
echo "Baseline run before dispatch: $BASELINE_RUN_ID"

# 本番デプロイをトリガー。
BUILD_TAG="build-$LATEST_COMMIT_SHA"
echo "Dispatching '$DISPATCH_WORKFLOW_FILE' with image_tag=$BUILD_TAG..."
gh workflow run "$DISPATCH_WORKFLOW_FILE" --ref "$TARGET_BRANCH" -f "image_tag=$BUILD_TAG"

echo ""
echo "Waiting for production deployment run to appear..."

# ベースラインより databaseId が大きい最新ランが我々の dispatch によるもの。
PROD_RUN_ID=""
for i in $(seq 1 "$MAX_CHECKS"); do
  CANDIDATE=$(gh run list \
    --workflow "$DISPATCH_WORKFLOW_FILE" \
    -L 1 \
    --json databaseId \
    --jq '.[0].databaseId // 0')
  if [[ "$CANDIDATE" -gt "$BASELINE_RUN_ID" ]]; then
    PROD_RUN_ID="$CANDIDATE"
    break
  fi
  echo "  [$i/$MAX_CHECKS] Waiting for new Production run to appear (baseline=$BASELINE_RUN_ID)..." >&2
  sleep "$POLL_INTERVAL"
done

if [[ -z "$PROD_RUN_ID" ]]; then
  echo "Timeout: Production run did not appear after dispatch" >&2
  exit 1
fi

echo "Found Production run: $PROD_RUN_ID"
gh run watch "$PROD_RUN_ID" --exit-status

echo ""
echo "Production deployment succeeded!"
