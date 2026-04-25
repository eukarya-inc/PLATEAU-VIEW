# PLATEAU Docs

PLATEAU 配信サービスの公式ドキュメントサイト。Astro + Starlight で構築され、[docs.plateauview.mlit.go.jp](https://docs.plateauview.mlit.go.jp) で公開中。

## 開発

```bash
npm install
npm run dev       # http://localhost:4321
npm run build     # → dist/
npm run preview
```

Node.js 22 以上を推奨。

## ディレクトリ

```
docs/
├ astro.config.mjs        # Starlight 設定（サイドバー、プラグイン）
├ src/
│  ├ assets/              # ロゴ等
│  ├ components/          # Starlight 上書き用コンポーネント
│  ├ content/docs/        # ドキュメント本体（MD/MDX）
│  ├ graphql/             # 取得した GraphQL スキーマ (SDL)
│  ├ openapi/             # OpenAPI 定義 (plateau-api.json)
│  └ styles/              # カスタム CSS
├ scripts/                # スキーマ取得・GraphQLドキュメント生成
├ public/                 # 静的ファイル（image.webp 等）
├ Dockerfile              # 静的サイトを nginx で配信
└ nginx.conf
```

## 主要な機能

- **Starlight**: サイドバー、検索 (Pagefind)、ダーク/ライトモード
- **starlight-llms-txt**: `/llms.txt`, `/llms-full.txt` 自動生成、各ページの `.md` 公開
- **starlight-openapi**: OpenAPI 定義からインタラクティブなリファレンスを生成

## スキーマの更新

本番 API から OpenAPI / GraphQL スキーマを取得してドキュメントに反映する。

```bash
npm run api:fetch     # OpenAPI を src/openapi/plateau-api.json に保存
npm run gql:build     # GraphQL スキーマ取得 + Markdown 生成
npm run schema:update # 上記をまとめて実行
```

## デプロイ

Docker イメージは `nginxinc/nginx-unprivileged:alpine` ベースの静的サイト配信で、Cloud Run に単独デプロイされている。`main` への push で GitHub Actions が自動ビルド・デプロイする。
