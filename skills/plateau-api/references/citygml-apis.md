# CityGML API

すべて `https://api.plateauview.mlit.go.jp` がベース。試験運用中。

## 1. 自治体ごとの CityGML 一覧

```
GET /datacatalog/plateau-datasets
```

レスポンスの `citygml[]` に自治体単位で 1 件ずつ。各自治体の全地物型を 1 つの zip にまとめた配信用 CityGML。

主要フィールド:
- `id`, `pref` / `pref_code`, `city` / `city_code`
- `url`: PLATEAU CMS 上の zip URL
- `file_size`: zip のバイト数（int64、null あり）
- `composite_url`: 自治体・整備年度を指定して zip にリダイレクトする安定 URL
- `feature_types`: 含まれる地物型コードのリスト
- `year`, `registration_year`, `spec`

加えて `latest_citygml[]` に各自治体の最新整備年度の zip リダイレクト URL（`year` は常に `"latest"`、`file_size` 同梱）。

`composite_url` / `latest_citygml[].url` は弱 ETag + `Cache-Control: no-cache, must-revalidate` を返す。`If-None-Match` 送付で 304 短絡可能。

GraphQL 版:

```bash
curl -fsSL -X POST https://api.plateauview.mlit.go.jp/datacatalog/graphql \
  -H 'Content-Type: application/json' \
  -d '{"query":"{ citygmlDatasets(input: { prefectureCodes: [\"13\"] }) { id cityCode url fileSize year featureTypes } }"}'
```

## 2. CityGML ファイル検索

```
GET /datacatalog/citygml/{conditions}
```

`{conditions}` プレフィックス:

| 種類 | プレフィックス | 例 |
|---|---|---|
| メッシュコード | `m:` | `m:533944,533945` |
| メッシュコード厳密 | `mm:` | `mm:533944` |
| 空間 ID | `s:` | `s:18/1/232853/103220` |
| 座標範囲（中心点） | `r:` | `r:139.7375,35.6583` |
| 座標範囲（矩形） | `r:` | `r:139.7375,35.6583,139.74,35.66` |
| ジオコーディング | `g:` | `g:千代田区` |
| 自治体コード | （なし） | `13999` |

```bash
curl -fsSL 'https://api.plateauview.mlit.go.jp/datacatalog/citygml/m:533935'
```

## 3. CityGML Pack（非同期 zip 生成）

| エンドポイント | 用途 |
|---|---|
| `POST /citygml/pack` | パック処理開始、`{ "id": "..." }` を返す |
| `GET /citygml/pack/{id}/status` | 状態（`accepted` / `processing` / `succeeded` / `failed`） |
| `GET /citygml/pack/{id}.zip` | 生成 zip ダウンロード |

```bash
curl -fsSL -X POST https://api.plateauview.mlit.go.jp/citygml/pack \
  -H 'Content-Type: application/json' \
  -d '{"urls":["https://assets.cms.plateau.reearth.io/assets/.../53394509_bldg_6697_op.gml","https://assets.cms.plateau.reearth.io/assets/.../53394518_bldg_6697_op.gml"]}'
```

制約:
- 投入できる URL は `assets.cms.plateau.reearth.io` のみ
- タイムアウトあり
- 生成 zip は一定時間後に削除される

## 4. CityGML 属性

```
GET /citygml/attributes?url={cityGmlUrl}&id={id1,id2,...}&skip_code_list_fetch={bool}
```

- `url`: CityGML ファイルの URL
- `id`: `gml:id` をカンマ区切り
- `skip_code_list_fetch`: `true` でコードリスト取得スキップ（生のコードを返す）

## 5. CityGML 空間 ID 検索

```
GET /citygml/features?url={cityGmlUrl}&sid={spatialId,...}
```

`sid`: カンマ区切り。`z/x/y` / `z/f/x/y` / ハッシュタイル形式。返却は `gml:id` リスト。

## 6. CityGML 空間 ID 属性（統合）

2 + 5 + 4 を 1 回で実行する統合 API。

```
GET /citygml/spatialid_attributes?sid={spatialId,...}&type={featureType,...}&skip_code_list_fetch={bool}
```

- `sid`: 空間 ID（カンマ区切り、最低 1 つ）
- `type`: 地物型コード（カンマ区切り、最低 1 つ、例: `bldg,veg,tran`）
- `skip_code_list_fetch`: コードリスト取得スキップ
