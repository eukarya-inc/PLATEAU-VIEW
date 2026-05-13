# 複合 URL 仕様（3D Tiles / MVT）

複数都市の 3D Tiles / MVT を 1 つの URL でまとめて取得するための動的生成エンドポイント。

## 3D Tiles 複合 tileset

```
GET https://api.plateauview.mlit.go.jp/datacatalog/3dtiles/{spec}/tileset.json
```

3D Tiles 仕様の external tileset 参照を使う。子 `tileset.json` は別ホスト（PLATEAU CMS）から配信される。

### `{spec}` 書式

```
<area>-<type>-<lod>[-interior][-<texture>]-<year>
```

| セグメント | 値 | 意味 |
|---|---|---|
| `area` | `all` | 全国 |
| | 2 桁数字 | 都道府県コード（`13` = 東京都） |
| | 5 桁数字 | 市区町村コード（区がある場合は区コード、無ければ市コード） |
| `type` | `bldg` / `tran` / `dem` 等 | データセット種別コード |
| `lod` | `lod<N>` | LOD が `<N>` と完全一致 |
| | `maxlod<N>` | LOD が `<N>` 以下で、各エリアごとに利用可能な最高 LOD を採用 |
| `interior` | （省略） | 屋外モデル（CityGML 3.0 以前のデータも含む） |
| | `interior` | CityGML 3.0 の屋内モデルのみ |
| `texture` | （省略） | テクスチャあり優先、無ければなしを採用 |
| | `texture` | テクスチャありのみ |
| | `notexture` | テクスチャなしのみ |
| `year` | 4 桁西暦 | その整備年度のみ |
| | `latest` | 整備年度フィルタなし、各自治体ごとに最新整備年度を採用 |

### 例

| spec | 意味 |
|---|---|
| `all-bldg-lod1-2025` | 全国の建築物 LOD1（2025 年度整備） |
| `all-bldg-maxlod2-2025` | 全国の建築物、各都市で LOD2 が取れるなら LOD2、無ければ LOD1 |
| `13-bldg-lod2-texture-2025` | 東京都の建築物 LOD2、テクスチャあり限定 |
| `13101-bldg-lod2-2025` | 千代田区の建築物 LOD2 |
| `all-bldg-lod2-latest` | 全国の建築物 LOD2、各自治体ごとに最新整備年度 |

## MVT TileJSON（自治体単位）

```
GET https://api.plateauview.mlit.go.jp/datacatalog/mvt/{spec}/tilejson.json
```

TileJSON 3.0 を返す。MapLibre GL JS / Mapbox GL JS 等で利用可能。

### `{spec}` 書式

```
<cityCode>-<type>[-lod<N>][-interior]-<year>
```

| セグメント | 値 | 意味 |
|---|---|---|
| `cityCode` | 5 桁数字 | 市区町村コード（区がある場合は区コード、無ければ市コード） |
| `type` | `luse` / `fld` 等 | データセット種別コード |
| `lod<N>` | （省略） | LOD 指定なしのデータセット |
| | `lod<N>` | LOD が `<N>` のデータセット |
| `interior` | （省略） | 屋外モデル |
| | `interior` | CityGML 3.0 の屋内モデルのみ |
| `year` | 4 桁西暦 | その整備年度のみ |
| | `latest` | 最新整備年度 |

### 例

| spec | 意味 |
|---|---|
| `13101-luse-2025` | 千代田区の土地利用 MVT（2025 年度整備） |
| `13101-luse-latest` | 千代田区の土地利用 MVT、最新整備年度 |
| `13101-fld-lod1-2025` | 千代田区の洪水浸水想定区域 MVT（LOD1） |
| `13101-bldg-lod3-interior-2025` | 千代田区の建築物 LOD3、屋内モデルのみ |

## REST レスポンス内の動的 URL 配列

`GET /datacatalog/plateau-datasets` のレスポンスに含まれる:

- `composite_tilesets`: 全国（`all-...`）と都道府県別（`13-...` 等）の複合 `tileset.json` を実データから派生して列挙。`-latest` エントリを含む。
- `latest_datasets`: 各 `(自治体, 種別, LOD, 形式)` で最新整備年度を採用する動的 URL 一覧。`year` は常に `"latest"`、`file_size`（バイト、int64）付き。
- `latest_citygml`: 各自治体の最新整備年度の CityGML zip にリダイレクトする URL 一覧。`file_size` 付き。

各 MVT 行の `composite_url` フィールドには TileJSON URL が入る。

## HTTP キャッシュ

複合 tileset.json / MVT tilejson.json / CityGML リダイレクトは:

- レスポンスボディ（リダイレクトの場合は転送先 URL）から導出した弱 ETag (`W/"..."`)
- `Cache-Control: no-cache, must-revalidate`

を返す。`If-None-Match` を送れば内容が変わっていなければ `304 Not Modified`。CMS 側でデータが差し替わると ETag が変わるので `-latest` URL でも安全に最新へ追従。
