---
name: plateau-api
description: PLATEAU 配信サービス（api.plateauview.mlit.go.jp）の REST / GraphQL API を使って、PLATEAU の 3D 都市モデル（3D Tiles, MVT, CityGML）データの配信 URL・データカタログ・属性情報を取得するときに使う。「PLATEAU の建築物データを取得したい」「3D Tiles の URL がほしい」「CityGML を絞り込んで取りたい」「データカタログ API」「複合 tileset.json」「PLATEAU GraphQL」などの場面で起動。
license: MIT
---

# PLATEAU 配信 API スキル

PLATEAU 配信サービスのデータ取得 API の使い方をまとめたスキル。エンドポイントは試験運用中で、スキーマ・URL・レスポンスは予告なく変更される。

## エンドポイント

| API | URL |
|---|---|
| REST | `https://api.plateauview.mlit.go.jp` |
| OpenAPI 定義 | `https://api.plateauview.mlit.go.jp/openapi.json` |
| GraphQL | `https://api.plateauview.mlit.go.jp/datacatalog/graphql`（ブラウザで開くと GraphiQL） |

公式ドキュメント:
- 一覧: <https://docs.plateauview.mlit.go.jp/>
- LLM 向け全文: <https://docs.plateauview.mlit.go.jp/llms-full.txt>

## どの API を使うか

| やりたいこと | 使う API |
|---|---|
| 全データセットの一覧をまとめて取りたい | REST `GET /datacatalog/plateau-datasets`（レスポンス ≒ 2MB+） |
| 特定の自治体・条件で絞り込みたい | GraphQL `area(code:...)` / `citygmlDatasets(input:...)` |
| 3D Tiles を CesiumJS で表示する URL がほしい | 複合 tileset URL（後述） or REST レスポンスの `composite_url` |
| MVT を MapLibre で表示したい | TileJSON URL（後述） or REST の `composite_url` |
| CityGML の zip をまとめて取得 | REST レスポンスの `citygml[].composite_url` / `latest_citygml` |
| CityGML から属性だけ抜きたい | CityGML 属性 API（`/citygml/attributes`） |
| 範囲指定で CityGML を検索 | `/datacatalog/citygml/{conditions}`（メッシュ/空間 ID/座標/自治体コード） |
| 複数 CityGML を 1 つの zip にまとめたい | CityGML Pack API（非同期） |

REST と GraphQL の使い分け:
- **一覧を一括取得**したいなら REST。1 リクエストで全データセットが返る。
- **必要なフィールドだけ取りたい / 自治体や種別で絞りたい**なら GraphQL。

## よく使う複合 URL の書式

### 3D Tiles 複合 tileset

```
GET /datacatalog/3dtiles/{spec}/tileset.json
```

`{spec}` = `<area>-<type>-<lod>[-interior][-<texture>]-<year>`

- `area`: `all` / 都道府県コード 2 桁 / 市区町村コード 5 桁
- `type`: `bldg` / `tran` / `dem` など
- `lod`: `lod<N>`（厳密） or `maxlod<N>`（N 以下で最高 LOD を採用）
- `interior`: 省略=屋外 / `interior`=CityGML 3.0 屋内のみ
- `texture`: 省略=テクスチャ優先 / `texture` / `notexture`
- `year`: 4 桁西暦 or `latest`（自治体ごとに最新整備年度を自動採用）

例:
- `all-bldg-lod1-2025` 全国建築物 LOD1（2025 年度整備）
- `13101-bldg-lod2-latest` 千代田区建築物 LOD2、各自治体ごと最新整備年度
- `13-bldg-lod2-texture-2025` 東京都建築物 LOD2 テクスチャあり限定

### MVT TileJSON

```
GET /datacatalog/mvt/{spec}/tilejson.json
```

`{spec}` = `<cityCode>-<type>[-lod<N>][-interior]-<year>`

例: `13101-luse-latest`, `13101-fld-lod1-2025`, `13101-bldg-lod3-interior-2025`

詳しい書式・全表は [references/composite-urls.md](references/composite-urls.md) を参照。

## CityGML API クイックリファレンス

| エンドポイント | 用途 |
|---|---|
| `GET /datacatalog/plateau-datasets` の `citygml[]` | 自治体単位の CityGML zip 一覧 |
| `GET /datacatalog/citygml/{conditions}` | 範囲検索（`m:`メッシュ / `s:`空間 ID / `r:`座標 / `g:`ジオコーディング / 自治体コード） |
| `POST /citygml/pack` → `GET /citygml/pack/{id}.zip` | 複数ファイルを zip にまとめる非同期 API |
| `GET /citygml/attributes` | `url` + `gml:id` で属性 JSON 取得 |
| `GET /citygml/features` | URL + 空間 ID で含まれる `gml:id` 一覧 |
| `GET /citygml/spatialid_attributes` | 上記 3 つを統合した空間 ID → 属性ショートカット |

詳細は [references/citygml-apis.md](references/citygml-apis.md) を参照。

## GraphQL の使い方

最小例:

```graphql
query {
  area(code: "13101") {
    id
    type
    datasets {
      id
      name
      items { id name url }
    }
  }
}
```

```bash
curl -X POST https://api.plateauview.mlit.go.jp/datacatalog/graphql \
  -H 'Content-Type: application/json' \
  --data '{"query":"{ area(code: \"13101\") { datasets { name items { url } } } }"}'
```

スキーマや追加クエリ例は [references/graphql.md](references/graphql.md) を参照。スキーマ全体はサーバの introspection または公式ドキュメントの「GraphQL スキーマリファレンス」で確認できる。

## 詳しく調べる手順

1. **このスキルでカバーされていない詳細**は `https://docs.plateauview.mlit.go.jp/llms-full.txt` を fetch して該当章を読む（全文 Markdown）。
2. **REST の正確な型定義・パラメータ**は OpenAPI 仕様 (`/openapi.json`) を参照。
3. **GraphQL のスキーマ全量**は GraphiQL の introspection を使う。
4. **MCP Server が利用可能な環境**では、`https://docs.plateauview.mlit.go.jp/mcp/overview/` の MCP 経由で取得した方が新鮮で正確。

## ファイルサイズ

各データセットアイテムには、CMS 上の実体ファイル（3D Tiles の zip / MVT の zip / CityGML の zip）のサイズが入る。REST は `file_size`（バイト、`int64`、null あり）、GraphQL は `fileSize`（`Int`、null あり）。事前のダウンロード見積りやプログレス表示に使える。

## 安定 URL の HTTP キャッシュ

`composite_url` / `latest_url` / `/datacatalog/3dtiles/...`, `/mvt/...`, `/citygml/...` の各エンドポイントは **弱 ETag + `Cache-Control: no-cache, must-revalidate`** を返す。クライアントは `If-None-Match` を付ければ 304 で短絡できる。データ差し替え時はサーバ側で ETag が変わるので自動で再取得される。

## 注意事項

- すべての API は **試験運用中**。スキーマ・URL・レスポンスは予告なく変更される。SLA なし。
- `/datacatalog/plateau-datasets` は **2MB 以上**になるので、モバイル回線では注意。gzip 対応。
- GraphQL は複雑すぎるクエリに制限がかかる。
- CityGML Pack API に投入できる URL は PLATEAU CMS（`assets.cms.plateau.reearth.io`）のファイルのみ。
- 生成された zip は一定時間で削除される。
