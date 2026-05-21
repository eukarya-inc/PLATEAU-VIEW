# Cloudflare Workers

PLATEAU の Cloudflare 上のエッジサービス群。Worker ごとにディレクトリを切る。

## Workers

| ディレクトリ | ドメイン | 役割 |
| --- | --- | --- |
| [`tiles`](./tiles) | `tiles.plateau.city` | R2 上のタイル配信。現状は `/terrain/*`（quantized-mesh）。将来 `/assets/*` 等を追加予定。 |

## 開発

各 worker ディレクトリで:

```bash
npm install        # 依存インストール
npm run cf-typegen # wrangler.toml から型生成 (worker-configuration.d.ts)
npm run dev        # ローカル開発 (wrangler dev)
npm run deploy     # デプロイ
```

デプロイ前に Cloudflare アカウントへログインしておくこと:

```bash
npx wrangler login
```

## CI / CD

- CI: `.github/workflows/ci-cloudflare-tiles.yml` — `cloudflare/tiles/**` 変更時に型チェック
- Deploy: `.github/workflows/deploy-cloudflare-tiles.yml` — `main` への push で CI 成功後に本番 (`wrangler deploy`) へ自動デプロイ

GitHub Actions Secrets に以下が必要:

- `CLOUDFLARE_API_TOKEN` — Workers / R2 への書き込み権限を持つ API トークン
- `CLOUDFLARE_ACCOUNT_ID` — 対象アカウントの ID

> 現状 dev / prod を分けていないため、`main` へのマージは即本番反映。dev 環境が必要になったら `wrangler.toml` の environments と `--env` フラグで分離する。

## URL マッピング (tiles)

- リクエスト: `https://tiles.plateau.city/terrain/<key>`
- R2 キー: `<key>`（例: `plateau-terrain-2024/0/0/0.terrain`）
- バケット: `plateau-terrain`
