#!/usr/bin/env bash
set -euo pipefail

# ===== 引数チェック =====
if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <server|worker>"
  echo ""
  echo "Examples:"
  echo "  $0 server    # Watch and deploy PLATEAU Server"
  echo "  $0 worker    # Watch and deploy PLATEAU Worker"
  exit 1
fi

TARGET_TYPE="$1"

# ===== 設定 =====
CI_WORKFLOW_NAME="CI"                              # 最初に待機するCIワークフロー
TARGET_BRANCH="main"                               # CIを見るブランチ
POLL_INTERVAL=10                                   # 何秒おきにチェックするか
MAX_CHECKS=60                                      # 最大チェック回数 (10秒x60=600秒=10分)

# ターゲットタイプに応じて設定を切り替え
case "$TARGET_TYPE" in
  server)
    DEV_WORKFLOW_NAME="⭐️ Deploy PLATEAU Server dev"
    DISPATCH_WORKFLOW_FILE="deploy-server-prod.yml"
    ;;
  worker)
    DEV_WORKFLOW_NAME="⭐️ Deploy PLATEAU Worker dev"
    DISPATCH_WORKFLOW_FILE="deploy-worker-prod.yml"
    ;;
  *)
    echo "Error: Invalid target type '$TARGET_TYPE'"
    echo "Must be either 'server' or 'worker'"
    exit 1
    ;;
esac

echo "Watching CI workflow '$CI_WORKFLOW_NAME' and Dev workflow '$DEV_WORKFLOW_NAME' on branch '$TARGET_BRANCH'..."
echo "On success, will dispatch '$DISPATCH_WORKFLOW_FILE'."
echo ""

# 最新コミットの日時を取得（Unix timestamp で比較）
LATEST_COMMIT_TIMESTAMP=$(git log -1 --format=%ct "$TARGET_BRANCH")
LATEST_COMMIT_DATE=$(git log -1 --format=%cI "$TARGET_BRANCH")
LATEST_COMMIT_SHA=$(git rev-parse "$TARGET_BRANCH")
echo "Latest commit on $TARGET_BRANCH:"
echo "  SHA: $LATEST_COMMIT_SHA"
echo "  Date: $LATEST_COMMIT_DATE (timestamp: $LATEST_COMMIT_TIMESTAMP)"
echo ""

# ===== Step 1: CI ワークフローの完了を待つ =====
echo "Step 1: Waiting for CI workflow to complete..."
echo ""

for i in $(seq 1 "$MAX_CHECKS"); do
  echo "[$i/$MAX_CHECKS] Checking CI workflow status..."

  # 最新の1件を取得
  RUN_INFO=$(gh run list \
    --workflow "$CI_WORKFLOW_NAME" \
    --branch "$TARGET_BRANCH" \
    -L 1 \
    --json status,conclusion,url,headSha,createdAt \
    --jq '.[0]')

  if [[ -z "$RUN_INFO" || "$RUN_INFO" == "null" ]]; then
    echo "  No workflow run found yet. Waiting..."
    sleep "$POLL_INTERVAL"
    continue
  fi

  STATUS=$(echo "$RUN_INFO" | jq -r '.status')
  CONCLUSION=$(echo "$RUN_INFO" | jq -r '.conclusion // ""')
  URL=$(echo "$RUN_INFO" | jq -r '.url')
  SHA=$(echo "$RUN_INFO" | jq -r '.headSha')
  CREATED_AT=$(echo "$RUN_INFO" | jq -r '.createdAt')

  echo "  status=$STATUS, conclusion=$CONCLUSION, sha=$SHA"
  echo "  created=$CREATED_AT"
  echo "  $URL"

  # ワークフローが最新コミットより前に実行されている場合は古いと判断
  # createdAt を Unix timestamp に変換して比較
  if command -v gdate &> /dev/null; then
    # macOS with GNU date (brew install coreutils)
    WORKFLOW_TIMESTAMP=$(gdate -d "$CREATED_AT" +%s)
  else
    # macOS with BSD date - UTC として扱う
    WORKFLOW_TIMESTAMP=$(TZ=UTC date -j -f "%Y-%m-%dT%H:%M:%SZ" "$CREATED_AT" +%s 2>/dev/null || date -d "$CREATED_AT" +%s 2>/dev/null || echo "0")
  fi

  # ワークフローのSHAが最新コミットと一致しているかチェック
  if [[ "$SHA" != "$LATEST_COMMIT_SHA" ]]; then
    echo ""
    echo "⚠️  This workflow run is for a different commit."
    echo "  Workflow SHA:  $SHA"
    echo "  Latest commit: $LATEST_COMMIT_SHA"
    echo "  Waiting for workflow for latest commit to start..."
    sleep "$POLL_INTERVAL"
    continue
  fi

  # まだ進行中
  if [[ "$STATUS" != "completed" ]]; then
    echo "  Workflow is still running. Waiting ${POLL_INTERVAL}s..."
    sleep "$POLL_INTERVAL"
    continue
  fi

  # completed かつ success → 次のステップへ
  if [[ "$CONCLUSION" == "success" ]]; then
    echo ""
    echo "✅ CI workflow succeeded for $SHA."
    echo ""
    break
  else
    echo ""
    echo "❌ CI workflow completed but not successful (conclusion=$CONCLUSION). Aborting."
    exit 1
  fi
done

if [[ "$CONCLUSION" != "success" ]]; then
  echo ""
  echo "⏰ Timeout: CI workflow did not complete successfully within limit (${MAX_CHECKS} checks)."
  exit 1
fi

# ===== Step 2: Deploy Dev ワークフローの完了を待つ =====
echo "Step 2: Waiting for Deploy Dev workflow to complete..."
echo ""

for i in $(seq 1 "$MAX_CHECKS"); do
  echo "[$i/$MAX_CHECKS] Checking Deploy Dev workflow status..."

  # 最新の1件を取得
  RUN_INFO=$(gh run list \
    --workflow "$DEV_WORKFLOW_NAME" \
    --branch "$TARGET_BRANCH" \
    -L 1 \
    --json status,conclusion,url,headSha,createdAt \
    --jq '.[0]')

  if [[ -z "$RUN_INFO" || "$RUN_INFO" == "null" ]]; then
    echo "  No workflow run found yet. Waiting..."
    sleep "$POLL_INTERVAL"
    continue
  fi

  STATUS=$(echo "$RUN_INFO" | jq -r '.status')
  CONCLUSION=$(echo "$RUN_INFO" | jq -r '.conclusion // ""')
  URL=$(echo "$RUN_INFO" | jq -r '.url')
  SHA=$(echo "$RUN_INFO" | jq -r '.headSha')
  CREATED_AT=$(echo "$RUN_INFO" | jq -r '.createdAt')

  echo "  status=$STATUS, conclusion=$CONCLUSION, sha=$SHA"
  echo "  created=$CREATED_AT"
  echo "  $URL"

  # ワークフローが最新コミットより前に実行されている場合は古いと判断
  # createdAt を Unix timestamp に変換して比較
  if command -v gdate &> /dev/null; then
    # macOS with GNU date (brew install coreutils)
    WORKFLOW_TIMESTAMP=$(gdate -d "$CREATED_AT" +%s)
  else
    # macOS with BSD date - UTC として扱う
    WORKFLOW_TIMESTAMP=$(TZ=UTC date -j -f "%Y-%m-%dT%H:%M:%SZ" "$CREATED_AT" +%s 2>/dev/null || date -d "$CREATED_AT" +%s 2>/dev/null || echo "0")
  fi

  # ワークフローのSHAが最新コミットと一致しているかチェック
  if [[ "$SHA" != "$LATEST_COMMIT_SHA" ]]; then
    echo ""
    echo "⚠️  This workflow run is for a different commit."
    echo "  Workflow SHA:  $SHA"
    echo "  Latest commit: $LATEST_COMMIT_SHA"
    echo "  Waiting for workflow for latest commit to start..."
    sleep "$POLL_INTERVAL"
    continue
  fi

  # まだ進行中
  if [[ "$STATUS" != "completed" ]]; then
    echo "  Workflow is still running. Waiting ${POLL_INTERVAL}s..."
    sleep "$POLL_INTERVAL"
    continue
  fi

  # completed かつ success → 次のステップへ
  if [[ "$CONCLUSION" == "success" ]]; then
    echo ""
    echo "✅ Deploy Dev workflow succeeded for $SHA."
    echo ""
    break
  else
    echo ""
    echo "❌ Deploy Dev workflow completed but not successful (conclusion=$CONCLUSION). Aborting."
    exit 1
  fi
done

if [[ "$CONCLUSION" != "success" ]]; then
  echo ""
  echo "⏰ Timeout: Deploy Dev workflow did not complete successfully within limit (${MAX_CHECKS} checks)."
  exit 1
fi

# ===== Step 3: 本番デプロイのトリガーと監視 =====
echo "Step 3: Triggering production deployment..."
echo ""

# 本番デプロイが既に実行されているかチェック
echo "Checking if production deployment is already completed..."

PROD_LATEST_RUN=$(gh run list \
  --workflow "$DISPATCH_WORKFLOW_FILE" \
  --branch "$TARGET_BRANCH" \
  -L 1 \
  --json status,conclusion,url,createdAt \
  --jq '.[0]')

if [[ -n "$PROD_LATEST_RUN" && "$PROD_LATEST_RUN" != "null" ]]; then
  PROD_LATEST_CREATED_AT=$(echo "$PROD_LATEST_RUN" | jq -r '.createdAt')
  PROD_LATEST_STATUS=$(echo "$PROD_LATEST_RUN" | jq -r '.status')
  PROD_LATEST_CONCLUSION=$(echo "$PROD_LATEST_RUN" | jq -r '.conclusion // ""')
  PROD_LATEST_URL=$(echo "$PROD_LATEST_RUN" | jq -r '.url')

  # UTC タイムスタンプに変換
  if command -v gdate &> /dev/null; then
    PROD_LATEST_TIMESTAMP=$(gdate -d "$PROD_LATEST_CREATED_AT" +%s)
  else
    PROD_LATEST_TIMESTAMP=$(TZ=UTC date -j -f "%Y-%m-%dT%H:%M:%SZ" "$PROD_LATEST_CREATED_AT" +%s 2>/dev/null || date -d "$PROD_LATEST_CREATED_AT" +%s 2>/dev/null || echo "0")
  fi

  echo "  Latest production run: $PROD_LATEST_CREATED_AT (timestamp: $PROD_LATEST_TIMESTAMP)"
  echo "  Latest commit:         $LATEST_COMMIT_DATE (timestamp: $LATEST_COMMIT_TIMESTAMP)"
  echo "  Status: $PROD_LATEST_STATUS, Conclusion: $PROD_LATEST_CONCLUSION"
  echo "  $PROD_LATEST_URL"

  # 本番デプロイが最新コミット以降に実行されていて、成功している場合
  if [[ "$PROD_LATEST_TIMESTAMP" -ge "$LATEST_COMMIT_TIMESTAMP" && "$PROD_LATEST_STATUS" == "completed" && "$PROD_LATEST_CONCLUSION" == "success" ]]; then
    echo ""
    echo "🎉 Production deployment is already completed for this commit!"
    echo "  No need to trigger again."
    exit 0
  fi
fi

echo ""
echo "Dispatching '$DISPATCH_WORKFLOW_FILE'..."

# Use the specific build tag for this commit to avoid caching issues with :latest
BUILD_TAG="build-$LATEST_COMMIT_SHA"
echo "Using image tag: $BUILD_TAG"

gh workflow run "$DISPATCH_WORKFLOW_FILE" --ref "$TARGET_BRANCH" -f "image_tag=$BUILD_TAG"

echo ""
echo "Production deployment has been triggered. Now watching for its completion..."
echo ""

# 本番デプロイの監視ループ
sleep 5  # 少し待ってからワークフローが表示されるようにする

for j in $(seq 1 "$MAX_CHECKS"); do
  echo "[$j/$MAX_CHECKS] Checking production deployment status..."

  PROD_RUN_INFO=$(gh run list \
    --workflow "$DISPATCH_WORKFLOW_FILE" \
    --branch "$TARGET_BRANCH" \
    -L 1 \
    --json status,conclusion,url,createdAt \
    --jq '.[0]')

  if [[ -z "$PROD_RUN_INFO" || "$PROD_RUN_INFO" == "null" ]]; then
    echo "  Production workflow not found yet. Waiting..."
    sleep "$POLL_INTERVAL"
    continue
  fi

  PROD_STATUS=$(echo "$PROD_RUN_INFO" | jq -r '.status')
  PROD_CONCLUSION=$(echo "$PROD_RUN_INFO" | jq -r '.conclusion // ""')
  PROD_URL=$(echo "$PROD_RUN_INFO" | jq -r '.url')
  PROD_CREATED_AT=$(echo "$PROD_RUN_INFO" | jq -r '.createdAt')

  echo "  status=$PROD_STATUS, conclusion=$PROD_CONCLUSION"
  echo "  $PROD_URL"

  # まだ進行中
  if [[ "$PROD_STATUS" != "completed" ]]; then
    echo "  Production deployment is still running. Waiting ${POLL_INTERVAL}s..."
    sleep "$POLL_INTERVAL"
    continue
  fi

  # completed かつ success
  if [[ "$PROD_CONCLUSION" == "success" ]]; then
    echo ""
    echo "🎉 Production deployment succeeded!"
    echo "  $PROD_URL"
    exit 0
  else
    echo ""
    echo "❌ Production deployment failed (conclusion=$PROD_CONCLUSION)."
    echo "  $PROD_URL"
    exit 1
  fi
done

echo ""
echo "⏰ Timeout: Production deployment did not complete within limit (${MAX_CHECKS} checks)."
exit 1
