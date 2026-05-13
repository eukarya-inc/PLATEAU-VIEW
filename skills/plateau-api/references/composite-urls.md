# 複合 URL 仕様（3D Tiles / MVT）

## 3D Tiles 複合 tileset.json

複数の都市の 3D Tiles を 1 つの URL でまとめて読み込むための動的生成エンドポイント。3D Tiles 仕様の external tileset 参照を使い、子タイルの `content.uri` で各都市の `tileset.json` を間接的に指す。

```
GET https://api.plateauview.mlit.go.jp/datacatalog/3dtiles/{spec}/tileset.json
```

### `{spec}` の書式

```
<area>-<type>-<lod>[-interior][-<texture>]-<year>
```

| セグメント | 値 | 意味 |
|---|---|---|
| `area` | `all` | 全国 |
| | 2 桁数字 | 都道府県コード（例: `13` = 東京都） |
| | 5 桁数字 | 市区町村コード（区がある場合は区コード、無ければ市コード） |
| `type` | `bldg` / `tran` / `dem` など | データセットの種別コード |
| `lod` | `lod<N>` | LOD が `<N>` と完全一致 |
| | `maxlod<N>` | LOD が `<N>` 以下で、各エリアごとに利用可能な最高 LOD を採用 |
| `interior` | （省略） | 屋外モデル（CityGML 3.0 以前のデータも含む） |
| | `interior` | CityGML 3.0 の屋内モデルのみ |
| `texture` | （省略） | テクスチャありを優先、無ければテクスチャなしを採用 |
| | `texture` | テクスチャありのみ |
| | `notexture` | テクスチャなしのみ |
| `year` | 4 桁西暦 | その整備年度のデータのみを採用 |
| | `latest` | 整備年度フィルタなし、各自治体ごとに最新整備年度を自動採用 |

### 例

| URL spec | 意味 |
|---|---|
| `all-bldg-lod1-2025` | 全国の建築物 LOD1（2025 年度整備） |
| `all-bldg-maxlod2-2025` | 全国の建築物、各都市で LOD2 が取れるなら LOD2、無ければ LOD1 |
| `13-bldg-lod2-texture-2025` | 東京都の建築物 LOD2、テクスチャあり限定 |
| `13101-bldg-lod2-2025` | 千代田区の建築物 LOD2 |
| `all-bldg-lod2-latest` | 全国の建築物 LOD2、各自治体ごとに最新整備年度 |

### CesiumJS での利用

```javascript
const tileset = await Cesium.Cesium3DTileset.fromUrl(
  "https://api.plateauview.mlit.go.jp/datacatalog/3dtiles/all-bldg-lod1-2025/tileset.json",
);
viewer.scene.primitives.add(tileset);
```

### 注意

- 子 `tileset.json` は別ホスト（PLATEAU CMS）から配信される。CesiumJS は自動で CORS 取得するが、まとめ読みは時間がかかることがある。
- API は試験運用中、書式は変わり得る。

## MVT TileJSON（自治体単位）

MapLibre GL JS / Mapbox GL JS など TileJSON ベースのクライアント向けに、自治体単位の **TileJSON 3.0** を動的生成する。

```
GET https://api.plateauview.mlit.go.jp/datacatalog/mvt/{spec}/tilejson.json
```

### `{spec}` の書式

```
<cityCode>-<type>[-lod<N>][-interior]-<year>
```

| セグメント | 値 | 意味 |
|---|---|---|
| `cityCode` | 5 桁数字 | 市区町村コード（区がある場合は区コード、無ければ市コード） |
| `type` | `luse` / `fld` など | データセットの種別コード |
| `lod<N>` | （省略） | LOD が指定されていないデータセットを採用 |
| | `lod<N>` | LOD が `<N>` のデータセットを採用 |
| `interior` | （省略） | 屋外モデル |
| | `interior` | CityGML 3.0 の屋内モデルのみ |
| `year` | 4 桁西暦 | その整備年度のみ |
| | `latest` | 最新整備年度 |

### 例

| URL spec | 意味 |
|---|---|
| `13101-luse-2025` | 千代田区の土地利用 MVT（2025 年度整備） |
| `13101-luse-latest` | 千代田区の土地利用 MVT、最新整備年度 |
| `13101-fld-lod1-2025` | 千代田区の洪水浸水想定区域 MVT（LOD1） |
| `13101-bldg-lod3-interior-2025` | 千代田区の建築物 LOD3、屋内モデルのみ |

`/datacatalog/plateau-datasets` のレスポンスでも、各 MVT 行の `composite_url` フィールドにこの TileJSON URL が入る。

## REST レスポンスに含まれる動的 URL 配列

`GET /datacatalog/plateau-datasets` のレスポンスには以下の補助配列が含まれる:

- `composite_tilesets`: 全国（`all-...`）と都道府県別（`13-...` など）の複合 `tileset.json` を実データから派生して列挙。`-latest` エントリも自動的に含まれる。
- `latest_datasets`: 各 `(自治体, 種別, LOD, 形式)` について最新整備年度を自動採用する動的 URL 一覧。`year` は常に `"latest"`。各エントリには `file_size`（参照先 zip のバイト数、`int64`）も含まれる。
- `latest_citygml`: 各自治体の最新整備年度の CityGML zip にリダイレクトする URL 一覧。`file_size` 付き。

## HTTP キャッシュ動作

複合 tileset.json / MVT tilejson.json / CityGML リダイレクトの各エンドポイントは:

- レスポンスボディ（リダイレクトの場合は転送先 URL）から導出した **弱 ETag** (`W/"..."`)
- `Cache-Control: no-cache, must-revalidate`

を返す。クライアントが `If-None-Match` を送れば、内容が変わっていなければ `304 Not Modified` で短絡する（ボディは送られない）。CMS 側でデータが差し替わると ETag が変わるので、`-latest` URL でも安全に最新データへ追従する。
