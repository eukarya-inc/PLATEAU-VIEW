# Cloudflare Workers

PLATEAU の Cloudflare 上のエッジサービス群。Worker ごとにディレクトリを切る。
npm の依存はリポジトリルートの `package.json`（workspaces: `cloudflare/*`）で一元管理する。

## Workers

| ディレクトリ | ドメイン | 役割 |
| --- | --- | --- |
| [`tiles`](./tiles) | `tiles.plateau.city` | R2 上のタイル/COG 配信。先頭パスでデータセット（バケット）を振り分け、任意キーの配信＋ディレクトリリスティング。 |

## URL マッピング (tiles)

1 つのホストが複数データセットを捌く。**先頭パスセグメント → バケットの対応は
`tiles/wrangler.toml` の `PATH_BUCKETS`（config）で決まり、コードにハードコードしない**。

| パス | バケット | 中身 |
| --- | --- | --- |
| `tiles.plateau.city/terrain/<key>` | `plateau-terrain` | quantized-mesh `.terrain` ＋ 地形 COG |
| `tiles.plateau.city/ortho/<key>` | `plateau-ortho` | オルソ COG（年度・都市別）|

- **オブジェクト配信**: `https://tiles.plateau.city/terrain/<key>` → 先頭セグメントを除いた
  `<key>` をバケットからそのまま返す（Range・条件付き・ETag 対応。`.terrain` は gzip、
  `.tif` は `image/tiff`）。
  例: `https://tiles.plateau.city/ortho/2024/11223_warabi-shi.tif`
- **ディレクトリリスティング**（動的カタログ）: パスが `/terrain/` のように `/` 終わり
  （またはデータセット名のみ）のとき、その直下を JSON で返す（CORS 付き。新規アップロード
  COG も自動反映）。`directories`/`files` のキーはバケット相対なので、URL は
  `/<dataset>/<key>` で組み立てる。1000 件超は `?cursor=` でページング。
  例: `https://tiles.plateau.city/ortho/2024/` → `{ dataset, prefix, directories, files, cursor }`
- **ルート** `https://tiles.plateau.city/` → 提供中のデータセット一覧（`{ datasets: ["terrain/","ortho/"] }`）。

### データセット追加手順（コード変更不要）

`tiles/wrangler.toml` に 2 点足すだけ:
1. `[[r2_buckets]]` に `binding = "FOO"` / `bucket_name = "…"`
2. `[vars].PATH_BUCKETS` に `foo = "FOO"`

## 開発

依存はルートで一括インストール（npm workspaces）。個々の worker には `-w` で:

```bash
npm install                            # ルートで全 worker の依存をインストール
npm run -w cloudflare/tiles cf-typegen # wrangler.toml から型生成 (worker-configuration.d.ts)
npm run -w cloudflare/tiles type       # 型生成 + 型チェック (tsc --noEmit)
npm run -w cloudflare/tiles dev        # ローカル開発 (wrangler dev)。実 R2 は --remote
npm run -w cloudflare/tiles deploy      # デプロイ (wrangler deploy)
```

デプロイ前に Cloudflare アカウントへログインしておくこと:

```bash
npx wrangler login
```

## CI / CD

- CI: `.github/workflows/ci-cloudflare-tiles.yml` — `cloudflare/tiles/**` 変更時に型チェック（ルートで `npm ci` → `-w cloudflare/tiles type`）
- Deploy: `.github/workflows/deploy-cloudflare-tiles.yml` — `main` への push で CI 成功後に本番 (`wrangler deploy`) へ自動デプロイ

GitHub Actions Secrets に以下が必要:

- `CLOUDFLARE_API_TOKEN` — Workers / R2 / Workers Routes (Zone) への Read+Write 権限を持つ、対象アカウント単一にスコープした API トークン

> 現状 dev / prod を分けていないため、`main` へのマージは即本番反映。ルーティングは
> `wrangler.toml` の `[[routes]]`（`tiles.plateau.city`、zone `plateau.city`）で割当。
