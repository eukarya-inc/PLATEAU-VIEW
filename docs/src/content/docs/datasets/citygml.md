---
title: PLATEAU-CityGML
description: PLATEAU の CityGML 形式データの配信仕様と CityGML API の利用方法
---

Project PLATEAU では、CityGML 形式で作成された 3D 都市モデルのデータの配信を行っており、Project PLATEAU が [G空間情報センター](https://front.geospatial.jp/plateau_portal_site/) において公開している都市の CityGML データを利用可能です。

ここでは、その CityGML データをより柔軟に取得するための **CityGML API** 群の使い方を、ユースケース別に説明します。

- API 共通エンドポイント: `https://api.plateauview.mlit.go.jp`

:::tip[詳細なリファレンス]
各エンドポイントの**パラメータ仕様・レスポンス構造・エラーコード・対話的なリクエスト実行**は [REST API リファレンス](/api/rest/) を参照してください。本ページは利用方法の解説に注力します。
:::

:::caution
この API は試験運用中であり、API のスキーマやレスポンスは予告なく変更されることがあります。また、提供期間やサービスレベルの保証もしておりません。クエリによってはレスポンスのサイズが大きくなることがあるため、モバイル回線などで使用する際は十分ご注意ください。
:::

## 1. 自治体ごとの CityGML 一覧 API

各自治体ごとに、すべての地物型を 1 つの zip ファイルにまとめた CityGML を一括取得できます。地物型ごとにファイルが分かれていない、配信用の統合された zip ファイルです。

[**`GET /datacatalog/plateau-datasets`**](/api/rest/operations/datacatalogplateau-datasets/) のレスポンスに含まれる `citygml` 配列に、自治体ごとの 1 件ずつの CityGML データセットが返却されます。

```sh
curl https://api.plateauview.mlit.go.jp/datacatalog/plateau-datasets | jq '.citygml[0]'
```

主要なフィールド: `id`, `pref` / `pref_code`, `city` / `city_code`, `url`（PLATEAU CMS 上の zip ファイル URL）, `composite_url`（自治体・整備年度を指定して zip にリダイレクトする安定 URL）, `feature_types`（含まれる地物型コードのリスト）, `year`, `registration_year`, `spec`。

:::tip[`url` ではなく `composite_url` の利用を推奨します]
PLATEAU の都市データは毎年更新され、`url` フィールドの CMS 直リンクは新しい年度のデータが公開されるとパスが変わる場合があります。`composite_url`（`/datacatalog/citygml/{cityCode}-{year|latest}/citygml.zip`）は API サーバ側で動的に解決するため、URL がより安定しています。さらに整備年度を `latest` に指定すれば、新しいデータが公開されたタイミングで URL を変更しなくても自動的に最新データに追従します。
:::

GraphQL からも同じ情報を取得できます。`citygmlDatasets` クエリで都道府県・市区町村・年度によるフィルタが可能です:

```graphql
query {
  citygmlDatasets(input: { prefectureCodes: ["13"] }) {
    id
    cityCode
    url
    year
    featureTypes
  }
}
```

:::tip
個別の CityGML ファイル（`bldg`、`tran` など地物型ごとに分かれた gml ファイル）が必要な場合は、続く CityGML ファイル検索 API を使用してください。
:::

## 2. CityGML ファイル検索 API

自治体コードやメッシュコードなど、様々な方法で指定した範囲に含まれる各種 CityGML ファイルの URL のリストを取得します。

[**`GET /datacatalog/citygml/{conditions}`**](/api/rest/operations/datacatalogcitygmlconditions/) — 検索条件 `conditions` には以下の形式が使えます:

| 種類 | プレフィックス | 例 |
|:---|:---|:---|
| メッシュコード | `m:` | `m:533944,533945` |
| メッシュコード厳密検索 | `mm:` | `mm:533944` |
| 空間 ID | `s:` | `s:18/1/232853/103220` |
| 座標範囲（中心点） | `r:` | `r:139.7375,35.6583` |
| 座標範囲（矩形） | `r:` | `r:139.7375,35.6583,139.74,35.66` |
| ジオコーディング | `g:` | `g:千代田区` |
| 自治体コード | （プレフィックスなし） | `13999` |

呼び出し例:

```sh
curl 'https://api.plateauview.mlit.go.jp/datacatalog/citygml/m:533935'
```

## 3. CityGML Pack API

複数の CityGML ファイル URL を投入すると、関連するファイルをまとめた zip ファイルを非同期で生成します。

| エンドポイント | 用途 |
|:---|:---|
| [`POST /citygml/pack`](/api/rest/operations/citygmlpack/) | パック処理を開始してパック ID を取得 |
| [`GET /citygml/pack/{id}/status`](/api/rest/operations/citygmlpackidstatus/) | パック処理の状態確認（`accepted` / `processing` / `succeeded` / `failed`） |
| [`GET /citygml/pack/{id}.zip`](/api/rest/operations/citygmlpackidzip/) | 生成された zip のダウンロード |

呼び出し例:

```sh
curl https://api.plateauview.mlit.go.jp/citygml/pack \
  --json '{
    "urls": [
      "https://assets.cms.plateau.reearth.io/assets/.../53394509_bldg_6697_op.gml",
      "https://assets.cms.plateau.reearth.io/assets/.../53394518_bldg_6697_op.gml"
    ]
  }'
```

サーバー負荷抑制のためタイムアウトなどの制限が設けられており、また一度作成された zip は一定時間経過後に削除されることがあります。投入できる URL は PLATEAU CMS（`assets.cms.plateau.reearth.io`）からのファイルのみです。

## 4. CityGML 属性 API

CityGML ファイルの URL と地物の `gml:id` を渡すと、その地物の属性情報を JSON で返します。

[**`GET /citygml/attributes`**](/api/rest/operations/citygmlattributes/) — クエリパラメータ:

- `url`: CityGML ファイルの URL
- `id`: `gml:id` をカンマ区切りで複数指定
- `skip_code_list_fetch`: コードリスト取得をスキップして生のコードを返す

## 5. CityGML 空間 ID 検索 API

CityGML の URL と空間 ID のリストを渡すと、その空間 ID 内に含まれる地物の `gml:id` のリストを返します。

[**`GET /citygml/features`**](/api/rest/operations/citygmlfeatures/) — クエリパラメータ:

- `url`: CityGML ファイルの URL
- `sid`: 空間 ID（カンマ区切り、`z/x/y`・`z/f/x/y`・ハッシュタイル形式）

## 6. CityGML 空間 ID 属性 API

空間 ID と地物型のリストを渡すと、該当地物の属性情報を JSON で返します。2・5・4 の API を順番に呼び出す手間を省く統合 API です。

[**`GET /citygml/spatialid_attributes`**](/api/rest/operations/citygmlspatialid_attributes/) — クエリパラメータ:

- `sid`: 空間 ID（カンマ区切り、最低 1 つ）
- `type`: 地物型（カンマ区切り、最低 1 つ。例: `bldg,veg,tran`）
- `skip_code_list_fetch`: コードリスト取得スキップ
