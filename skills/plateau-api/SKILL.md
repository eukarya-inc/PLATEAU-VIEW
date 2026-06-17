---
name: plateau-api
description: PLATEAU 配信サービス（api.plateauview.mlit.go.jp）の REST / GraphQL API を使って、PLATEAU の 3D 都市モデル（3D Tiles, MVT, CityGML）データの配信 URL・データカタログ・属性情報を取得するときに使う。「PLATEAU の建築物データを取得したい」「3D Tiles の URL がほしい」「CityGML を絞り込んで取りたい」「データカタログ API」「複合 tileset.json」「PLATEAU GraphQL」などの場面で起動。
license: MIT
---

# PLATEAU 配信 API

PLATEAU 3D 都市モデルの配信 API。試験運用中で、スキーマ・URL・レスポンスは予告なく変更される。

## エンドポイント

- REST: `https://api.plateauview.mlit.go.jp`
- OpenAPI 3.0: `https://api.plateauview.mlit.go.jp/openapi.json`
- GraphQL: `https://api.plateauview.mlit.go.jp/datacatalog/graphql`

## API 選択

| やりたいこと | 使う API |
|---|---|
| 全データセット一覧（2MB+） | REST `GET /datacatalog/plateau-datasets` |
| 自治体・条件で絞り込み | GraphQL（[references/graphql.md](references/graphql.md)） |
| 3D Tiles 表示 URL | 複合 tileset（[references/composite-urls.md](references/composite-urls.md)） |
| MVT TileJSON | [references/composite-urls.md](references/composite-urls.md) |
| CityGML 検索 / Pack / 属性 / 空間 ID | [references/citygml-apis.md](references/citygml-apis.md) |
| 仕様書（標準製品仕様書・標準作業手順書）の全文検索 / 目次 / 本文 | [references/spec-apis.md](references/spec-apis.md) |

REST レスポンスの主要フィールド（`citygml[]` / `latest_datasets[]` / `latest_citygml[]` 等）:
`url`, `composite_url`, `file_size`（int64、null あり）, `year`, `pref_code`, `city_code`, `feature_types`。

## スキーマを取得する

REST（OpenAPI 3.0 JSON）:

```bash
curl -fsSL https://api.plateauview.mlit.go.jp/openapi.json
```

GraphQL（introspection）:

```bash
curl -fsSL -X POST https://api.plateauview.mlit.go.jp/datacatalog/graphql \
  -H 'Content-Type: application/json' \
  -d '{"query":"query IntrospectionQuery { __schema { queryType { name } mutationType { name } subscriptionType { name } types { ...FullType } directives { name description locations args { ...InputValue } } } } fragment FullType on __Type { kind name description fields(includeDeprecated: true) { name description args { ...InputValue } type { ...TypeRef } isDeprecated deprecationReason } inputFields { ...InputValue } interfaces { ...TypeRef } enumValues(includeDeprecated: true) { name description isDeprecated deprecationReason } possibleTypes { ...TypeRef } } fragment InputValue on __InputValue { name description type { ...TypeRef } defaultValue } fragment TypeRef on __Type { kind name ofType { kind name ofType { kind name ofType { kind name ofType { kind name ofType { kind name ofType { kind name ofType { kind name } } } } } } } }"}'
```

最新のドキュメント全文（人間向けの解説含む）:

```bash
curl -fsSL https://docs.plateauview.mlit.go.jp/llms-full.txt
```

## HTTP キャッシュ

複合 tileset.json / MVT tilejson.json / CityGML リダイレクトの各エンドポイントは弱 ETag (`W/"..."`) と `Cache-Control: no-cache, must-revalidate` を返す。`If-None-Match` を送れば `304 Not Modified` で短絡する。`-latest` URL でも内容が変わらなければ 304、差し替わると ETag が更新されて新しいデータに追従する。

## 注意

- API は試験運用中。スキーマ・URL・レスポンスは予告なく変更され得る。SLA なし。
- `/datacatalog/plateau-datasets` のレスポンスは 2MB 以上。gzip 対応。
- GraphQL は複雑すぎるクエリに制限あり。
- CityGML Pack API に投入できる URL は PLATEAU CMS（`assets.cms.plateau.reearth.io`）のファイルのみ。生成 zip は一定時間後に削除される。
