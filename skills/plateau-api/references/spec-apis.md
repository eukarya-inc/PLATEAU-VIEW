# 仕様書 API（標準製品仕様書 / 標準作業手順書）

PLATEAU 3D 都市モデルの **標準製品仕様書** と **標準作業手順書** を全文検索・目次取得・本文取得できる REST API。MCP（`plateau_spec_*` ツール）と同じ機能を、MCP クライアントなしで利用できる。

ベース URL: `https://api.plateauview.mlit.go.jp`

`{document_type}`（対象文書）:

- `standard`: 3D都市モデル標準製品仕様書
- `procedure`: 3D都市モデル標準作業手順書

リソース指向の構成:

| 用途 | エンドポイント |
|---|---|
| 文書一覧 | `GET /spec` |
| 全文検索 | `GET /spec/search?q=...` |
| 目次（index） | `GET /spec/{document_type}` |
| 本文（get） | `GET /spec/{document_type}/{path}` |

## 1. 文書一覧

```bash
curl -fsSL 'https://api.plateauview.mlit.go.jp/spec'
```

```json
[
  {"document_type":"standard","title":"3D都市モデル標準製品仕様書","path":"/spec/standard"},
  {"document_type":"procedure","title":"3D都市モデル標準作業手順書","path":"/spec/procedure"}
]
```

## 2. 全文検索

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

レスポンス（JSON）。`path` と `document_type` をそのまま本文取得に渡せる:

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

## 3. 目次（index）

```
GET /spec/{document_type}
```

| パラメータ | 必須 | 説明 |
|---|---|---|
| `depth` | | 階層の深さ。1=章のみ、2=章+節（既定: 2、最大: 4） |
| `chapter` | | 特定の章だけ取得（例: `toc4`） |
| `format` | | `json`（既定） / `markdown` |

```bash
curl -fsSL 'https://api.plateauview.mlit.go.jp/spec/standard?depth=1'
```

`format=json` のレスポンスは目次項目の配列（`id` / `title` / `path` / `children[]`）。

## 4. 本文（get）

```
GET /spec/{document_type}/{path}
```

| パラメータ | 必須 | 説明 |
|---|---|---|
| `single_page` | | `true` で子セクションを含めず単一ページのみ（既定: `false`） |
| `format` | | `markdown`（既定、`text/markdown` で本文を返す） / `json`（`{path, document_type, content}`） |

`{path}` は目次・検索で得たパス（例: `toc1`, `toc4_01`）。末尾に拡張子を付けると出力形式を選べる（`format` クエリより優先）:

- `/spec/standard/toc1` または `/spec/standard/toc1.md` → Markdown
- `/spec/standard/toc1.json` → JSON

```bash
curl -fsSL 'https://api.plateauview.mlit.go.jp/spec/standard/toc1'        # Markdown
curl -fsSL 'https://api.plateauview.mlit.go.jp/spec/standard/toc1.json'   # JSON
```

既定では指定セクション配下の子ページも含めて Markdown で返す。範囲が広いと本文が大きくなるため、必要に応じて目次でより深いパスに絞る。

## 典型的な流れ

1. `/spec/search?q=...` でキーワードに該当するセクションの `document_type` と `path` を見つける
2. `/spec/{document_type}/{path}` で本文を読む
3. 全体像を把握したいときは `/spec/{document_type}` で目次を取得
