# PLATEAU Docs

PLATEAU 配信サービスの公式ドキュメントサイト。Astro + Starlight で構築され、`docs.plateauview.mlit.go.jp` で公開予定。

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
│  ├ content/docs/        # ドキュメント本体（MD/MDX）
│  └ openapi/             # OpenAPI 定義（追加予定）
├ public/                 # 静的ファイル（image.webp 等）
├ Dockerfile              # 静的サイトを nginx で配信
└ nginx.conf
```

## 主要な機能

- **Starlight**: サイドバー、検索 (Pagefind)、ダーク/ライトモード
- **starlight-llms-txt**: `/llms.txt`, `/llms-full.txt` 自動生成、各ページの `.md` 公開
- **@scalar/starlight-openapi**: OpenAPI 定義からインタラクティブなリファレンスを生成

## デプロイ

Docker イメージは `nginxinc/nginx-unprivileged:alpine` ベースの静的サイト配信。Cloud Run に単独デプロイする想定。
