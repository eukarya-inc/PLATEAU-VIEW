# 仕様書 API（標準製品仕様書 / 標準作業手順書）

PLATEAU 3D 都市モデルの **標準製品仕様書** と **標準作業手順書** を全文検索・目次取得・本文取得できる REST API。MCP（`plateau_spec_*` ツール）と同じ機能を、MCP クライアントなしで利用できる。

ベース URL: `https://api.plateauview.mlit.go.jp`

`document_type` は全エンドポイント共通:

- `standard`: 3D都市モデル標準製品仕様書（既定）
- `procedure`: 3D都市モデル標準作業手順書
- `all`: 両方（検索のみ）

## 1. 全文検索

```
GET /spec/search
```

| パラメータ | 必須 | 説明 |
|---|---|---|
| `q` | ✓ | 検索クエリ。日本語・英語に対応（例: `LOD`, `CityGML`, `属性`, `メタデータ`） |
| `document_type` | | `standard` / `procedure` / `all`（既定: `all`） |
| `limit` | | 最大取得件数（既定: 10、最大: 50） |

```bash
curl -fsSL 'https://api.plateauview.mlit.go.jp/spec/search?q=LOD&document_type=all&limit=5'
```

レスポンス（JSON）:

```json
{
  "query": "LOD",
  "results": [
    {
      "title": "4.1 データ構造",
      "path": "toc4_01",
      "document_type": "standard",
      "score": 1.52,
      "snippets": ["LOD は地物の詳細度を表す ..."]
    }
  ]
}
```

`path` を `/spec/read` に渡すと本文を取得できる。

## 2. 目次（アウトライン）

```
GET /spec/outline
```

| パラメータ | 必須 | 説明 |
|---|---|---|
| `document_type` | | `standard` / `procedure`（既定: `standard`） |
| `depth` | | 階層の深さ。1=章のみ、2=章+節（既定: 2、最大: 4） |
| `chapter` | | 特定の章だけ取得（例: `toc4`） |
| `format` | | `json`（既定） / `markdown` |

```bash
curl -fsSL 'https://api.plateauview.mlit.go.jp/spec/outline?depth=1'
```

`format=json` のレスポンスは目次項目の配列（`id` / `title` / `path` / `children[]`）。

## 3. 本文取得

```
GET /spec/read
```

| パラメータ | 必須 | 説明 |
|---|---|---|
| `path` | ✓ | 取得するパス（例: `toc1`, `/plateaudocument/toc4`）。目次・検索で取得 |
| `document_type` | | `standard` / `procedure`（既定: `standard`） |
| `single_page` | | `true` で子セクションを含めず単一ページのみ（既定: `false`） |
| `format` | | `markdown`（既定、`text/markdown` で本文を返す） / `json`（`{path, document_type, content}`） |

```bash
curl -fsSL 'https://api.plateauview.mlit.go.jp/spec/read?path=toc1'
```

既定では指定セクション配下の子ページも含めて Markdown で返す。範囲が広いと本文が大きくなるため、必要に応じて `chapter` で章を絞るか、より深いパスを指定する。

## 典型的な流れ

1. `/spec/search?q=...` でキーワードに該当するセクションの `path` を見つける
2. `/spec/read?path=<path>` で本文を読む
3. 全体像を把握したいときは `/spec/outline` で目次を取得
