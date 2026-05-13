# CityGML API リファレンス

すべて `https://api.plateauview.mlit.go.jp` がベース。試験運用中で予告なく変更され得る。

## 1. 自治体ごとの CityGML 一覧

各自治体の全地物型を 1 つの zip にまとめた配信用 CityGML を取得する。

```
GET /datacatalog/plateau-datasets
```

レスポンスの `citygml[]` 配列に自治体単位で 1 件ずつ入る。

主要フィールド:
- `id`, `pref` / `pref_code`, `city` / `city_code`
- `url`: PLATEAU CMS 上の zip ファイル URL
- `file_size`: zip のファイルサイズ（バイト、`int64`、CMS の totalSize 由来）
- `composite_url`: 自治体・整備年度を指定して zip にリダイレクトする安定 URL
- `feature_types`: 含まれる地物型コードのリスト
- `year`, `registration_year`, `spec`

加えて `latest_citygml[]` に「各自治体の最新整備年度の zip」へのリダイレクト URL（`year` は常に `"latest"`、`file_size` 同梱）。

`composite_url` / `latest_citygml[].url` への GET は **弱 ETag + `Cache-Control: no-cache, must-revalidate`** を返す（参照先が変わると ETag も変わる）。クライアントが `If-None-Match` を送れば、リダイレクト先が同じなら `304` で短絡する。

### GraphQL でも取れる

```graphql
query {
  citygmlDatasets(input: { prefectureCodes: ["13"] }) {
    id
    cityCode
    url
    fileSize
    year
    featureTypes
  }
}
```

`fileSize`（バイト、`Int`、null あり）は CMS の totalSize 由来。`DatasetItem` 系（`PlateauDatasetItem` / `GenericDatasetItem` / `RelatedDatasetItem`）にも同じ `fileSize` フィールドがある。`RelatedDatasetItem` は変換前データの `originalFileSize` も持つ。

## 2. CityGML ファイル検索

範囲・条件を指定して CityGML ファイル URL のリストを取得。

```
GET /datacatalog/citygml/{conditions}
```

`{conditions}` のプレフィックス:

| 種類 | プレフィックス | 例 |
|---|---|---|
| メッシュコード | `m:` | `m:533944,533945` |
| メッシュコード厳密検索 | `mm:` | `mm:533944` |
| 空間 ID | `s:` | `s:18/1/232853/103220` |
| 座標範囲（中心点） | `r:` | `r:139.7375,35.6583` |
| 座標範囲（矩形） | `r:` | `r:139.7375,35.6583,139.74,35.66` |
| ジオコーディング | `g:` | `g:千代田区` |
| 自治体コード | （なし） | `13999` |

例:
```bash
curl 'https://api.plateauview.mlit.go.jp/datacatalog/citygml/m:533935'
```

## 3. CityGML Pack（非同期 zip 生成）

複数の CityGML URL を投入し、まとめた zip を生成する。

| エンドポイント | 用途 |
|---|---|
| `POST /citygml/pack` | パック処理開始、パック ID を返す |
| `GET /citygml/pack/{id}/status` | 状態（`accepted` / `processing` / `succeeded` / `failed`） |
| `GET /citygml/pack/{id}.zip` | 生成された zip ダウンロード |

例:
```bash
curl https://api.plateauview.mlit.go.jp/citygml/pack \
  --json '{
    "urls": [
      "https://assets.cms.plateau.reearth.io/assets/.../53394509_bldg_6697_op.gml",
      "https://assets.cms.plateau.reearth.io/assets/.../53394518_bldg_6697_op.gml"
    ]
  }'
```

制約:
- 投入できるのは PLATEAU CMS (`assets.cms.plateau.reearth.io`) のファイルのみ
- タイムアウトあり、生成 zip は一定時間後に削除される

## 4. CityGML 属性

CityGML ファイル URL と `gml:id` を渡し、属性 JSON を取得。

```
GET /citygml/attributes?url=...&id=ID1,ID2&skip_code_list_fetch=false
```

- `url`: CityGML ファイルの URL
- `id`: `gml:id` をカンマ区切り
- `skip_code_list_fetch`: true でコードリスト取得をスキップ（生のコードを返す）

## 5. CityGML 空間 ID 検索

URL と空間 ID から、その空間 ID に含まれる地物の `gml:id` リストを返す。

```
GET /citygml/features?url=...&sid=...
```

- `sid`: カンマ区切り。`z/x/y` / `z/f/x/y` / ハッシュタイル形式

## 6. CityGML 空間 ID 属性（統合）

2 + 5 + 4 を 1 回でやる統合 API。

```
GET /citygml/spatialid_attributes?sid=...&type=...&skip_code_list_fetch=false
```

- `sid`: 空間 ID（カンマ区切り、最低 1 つ）
- `type`: 地物型（カンマ区切り、最低 1 つ、例: `bldg,veg,tran`）
- `skip_code_list_fetch`: コードリスト取得スキップ
