---
title: PLATEAU-3DTiles / MVT
description: 3D Tiles および MVT 形式での建築物モデル等の配信仕様と利用方法
---

## 1. PLATEAU-3DTiles / MVT の概要

Project PLATEAU では、CityGML 形式で作成された 3D 都市モデルのデータを 3D Tiles および MVT（Mapbox Vector Tiles）形式に変換し、それぞれ配信を行っています。3D Tiles の形式バージョンは従来 1.0 ですが、2025 年度以降に整備されたデータの一部は 1.1 で配信されます。

本チュートリアルでは、3D Tiles および MVT の利用方法について解説します。

:::tip
3D Tiles 形式の 3D 都市モデルデータの仕様については、[PLATEAU 2023 可視化用データ変換仕様](/spec/visualization/) をご覧ください。
:::

## 2. CityGML の 3D Tiles / MVT への変換

CityGML を 3D Tiles または MVT 形式に変換するためには、以下の方法があります。

- [PLATEAU GIS Converter](https://github.com/Project-PLATEAU/PLATEAU-GIS-Converter): FY2023 の Project PLATEAU「都市デジタルツインの実現に向けた研究開発及び実証調査業務」（内閣府/研究開発と Society5.0 との橋渡しプログラム（BRIDGE））において開発された OSS の変換ツール
- [FME](https://github.com/Project-PLATEAU/FMEscript-CityGML-to-3DTiles): Project PLATEAU で利用した CityGML から 3D Tiles への FME 変換テンプレート
- [Cesium ion](https://cesium.com/learn/3d-tiling/ion-tile-3d-buildings/): オンラインサービスを利用したデータ変換とホスティング
- [Cesium ion オンプレミス](https://cesium.com/platform/on-premises-products/): 3D Tiles 変換用の有償のコマンドラインツール
- [citygml-to-3dtiles](https://github.com/njam/citygml-to-3dtiles): データ変換用オープンソースツール

## 3. PLATEAU-3DTiles / MVT の利用方法

PLATEAU-3DTiles / MVT の配信サービスを利用することで、独自に立ち上げた CesiumJS 等の環境で 3D 都市モデルをすぐに利用することができます。詳しくは、本ドキュメント末尾の「配信データの取得方法」をご覧ください。

なお、本サービスはあくまで試験的な運用であるため、提供期間やサービスレベルについては保証できないことをご了承ください。またデータの内容は予告なく更新されることがあります。

### 3.1. CesiumJS での利用方法

3D Tiles / MVT 形式のデータは、PLATEAU VIEW 上でデータカタログから選択したり、CesiumJS を利用したコードを作成したりすることで表示できます。ただし、MVT は現在 CesiumJS ではサポートされていないため、別途外部ライブラリが必要になります（ここでは [cesium-mvt-imagery-provider](https://github.com/reearth/cesium-mvt-imagery-provider) を使用）。

以下は、CesiumJS を利用して 3D Tiles を表示させるためのサンプルコードです。この例では、東京都千代田区の建築物モデル（3D Tiles）を利用していますが、下記の「配信データの取得方法」にある URL に置き換えることで、様々なデータを表示することができます。

```html
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <title>PLATEAU-3DTiles/MVT、PLATEAU-Ortho、PLATEAU-Terrain を Cesium で表示</title>
  <script src="https://cesium.com/downloads/cesiumjs/releases/1.117/Build/Cesium/Cesium.js"></script>
  <link href="https://cesium.com/downloads/cesiumjs/releases/1.117/Build/Cesium/Widgets/widgets.css" rel="stylesheet">
  <script src="https://unpkg.com/cesium-mvt-imagery-provider@1.4.1/dist/cesium-mvt-imagery-provider.umd.js"></script>
  <style>
    #cesiumContainer {
      position: absolute; top: 0; left: 0; height: 100%; width: 100%;
      margin: 0; overflow: hidden; padding: 0; font-family: sans-serif;
    }
    html { height: 100%; }
    body { padding: 0; margin: 0; overflow: hidden; height: 100%; }
  </style>
</head>
<body>
  <div id="cesiumContainer"></div>
  <script>
    // PLATEAU-Terrain で必要
    Cesium.Ion.defaultAccessToken = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiJlNjk0MTM4NC1lMWI0LTQxNTgtYjcxZS01ZWJhMGJlMTE1MWQiLCJpZCI6MTQ5ODk3LCJpYXQiOjE3MTUxNTEyODZ9.2aUmEQ2-fDsjf-XeC6-hZpwkgwLse3yXoXF4xTOvPAY";

    const viewer = new Cesium.Viewer("cesiumContainer", {});

    // PLATEAU-Terrain
    viewer.scene.setTerrain(
      new Cesium.Terrain(
        Cesium.CesiumTerrainProvider.fromIonAssetId(2488101),
      ),
    );

    // PLATEAU-Ortho
    const imageProvider = new Cesium.UrlTemplateImageryProvider({
      url: 'https://tile.plateauview.mlit.go.jp/tiles/plateau-ortho-2023/{z}/{x}/{y}.png',
      maximumLevel: 19
    });
    viewer.scene.imageryLayers.addImageryProvider(imageProvider);

    // 東京都千代田区の建築物モデル（3D Tiles, 複合 tileset.json）
    Cesium.Cesium3DTileset.fromUrl(
      'https://api.plateauview.mlit.go.jp/datacatalog/3dtiles/13101-bldg-lod2-latest/tileset.json'
    ).then((tileset) => {
      viewer.scene.primitives.add(tileset);
    });

    // 東京都千代田区の土地利用モデル（MVT, TileJSON 経由）
    fetch('https://api.plateauview.mlit.go.jp/datacatalog/mvt/13101-luse-latest/tilejson.json')
      .then(r => r.json())
      .then(tj => {
        const yourMvt = new CesiumMVTImageryProvider.CesiumMVTImageryProvider({
          urlTemplate: tj.tiles[0],
          layerName: tj.vector_layers[0].id,
          style: feature => ({ fillStyle: "white" }),
        });
        viewer.scene.imageryLayers.addImageryProvider(yourMvt);
      });

    // カメラの初期位置の指定
    viewer.camera.setView({
      destination: Cesium.Cartesian3.fromDegrees(139.76, 35.68, 5000.0)
    });
  </script>
</body>
</html>
```

## 4. 配信データの取得方法

Project PLATEAU が [G空間情報センター](https://www.geospatial.jp/ckan/dataset/plateau) において公開している都市の 3D Tiles および MVT データを利用可能です。

配信データを取得するための API として、以下の 2 種類の「データカタログ API」を公開しています。

- 通常の Web API
- GraphQL API

:::tip[詳細なリファレンス]
各エンドポイントの**パラメータ仕様・レスポンス構造・対話的なリクエスト実行**は API リファレンスを参照してください。本ページは利用方法の概観に注力します。
- REST: [`GET /datacatalog/plateau-datasets`](/api/rest/operations/datacatalogplateau-datasets/)
- GraphQL: [スキーマリファレンス](/api/graphql/schema/) / [プレイグラウンド](/api/graphql/playground/)
:::

### 4.1. 通常の Web API（REST）

[**`GET /datacatalog/plateau-datasets`**](/api/rest/operations/datacatalogplateau-datasets/) を呼び出すと、3D Tiles / MVT のデータ一覧が JSON で返却されます。レスポンス内の `url` フィールドに含まれる URL を CesiumJS 等で利用してください。

```sh
curl https://api.plateauview.mlit.go.jp/datacatalog/plateau-datasets
```

主要なフィールド: `name`, `pref` / `pref_code`, `city` / `city_code`, `ward` / `ward_code`, `type` / `type_en`, `url`, `file_size`（`url` 先 zip のファイルサイズ（バイト）。CMS のメタデータが取得できる場合に提供）, `composite_url`（3D Tiles の場合は複合 `tileset.json`、MVT の場合は自治体単位の `tilejson.json` への直リンク）, `layers`（MVT のみ）, `year`, `registration_year`, `spec`, `format`（`3D Tiles` または `MVT`）, `format_version`（3D Tiles の場合のみ。`1.0` または `1.1`。2025年度以降に Flow で変換されたデータは `1.1`、それ以外は `1.0`）, `lod`, `texture`。型と意味の一覧は [REST API リファレンス](/api/rest/operations/datacatalogplateau-datasets/) を参照してください。

:::tip[`url` ではなく `composite_url` の利用を推奨します]
PLATEAU の都市データは毎年更新され、新しい整備年度のデータが公開されます。`url` フィールドは CMS 上の特定アセットへの直リンクのため、新しい年度のデータが公開されてもアプリケーション側で URL を書き換えない限り古いデータを参照し続けます。

`composite_url` は API サーバ側でデータセットを動的に解決して `tileset.json` / `tilejson.json` を返すため、URL がより安定しています。さらに、後述の [複合 tileset.json](#42-複合-tilesetjson複数都市の-3d-tiles-を-1-つの-url-でまとめて取得) や [MVT TileJSON](#43-自治体単位の-mvt-tilejsonmaplibre-などから利用) で **整備年度を `latest` に指定した形**（例: `13101-bldg-lod2-latest`、`13101-luse-latest`）を使えば、新しい整備年度のデータが公開されたタイミングで URL を変更しなくても自動的に最新データに追従します。
:::

レスポンスにはさらに次の配列が含まれます。

- `composite_tilesets`: 全国（`all-...`）と都道府県別（`13-...` など）の複合 `tileset.json` を実データから派生して列挙したリスト。CesiumJS で広域をまとめて表示したい場合の入口として利用できます。年度別エントリと「最新整備年度」を自動採用する `-latest` エントリの両方を含みます。
- `latest_datasets`: 各 `(自治体, 種別, LOD, 形式)` について **最新整備年度** を自動採用する 3D Tiles / MVT 用の動的 URL 一覧。`year` は常に `"latest"`。新しい整備年度が公開されたタイミングで URL を変更しなくても自動的に最新データに追従します。
- `latest_citygml`: 各自治体について **最新整備年度** の CityGML zip にリダイレクトする URL 一覧。同様に `year` は `"latest"`。

加えて `citygml` 配列には、自治体ごとの CityGML データセットが含まれます。地物型ごとには分かれず、各自治体の全地物型を 1 つの zip ファイルにまとめた形で、自治体単位で 1 件ずつ返却されます。詳細は [PLATEAU-CityGML](/datasets/citygml/) を参照してください。

:::caution
レスポンスサイズは約 2 MB 以上あります。API は gzip 圧縮に対応しているものの、モバイル回線では十分ご注意ください。手早くデータを探したいだけなら [データセット一覧](/datasets/explorer/) ページがおすすめです。
:::

### 4.2. 複合 tileset.json（複数都市の 3D Tiles を 1 つの URL でまとめて取得）

複数の都市の 3D Tiles を CesiumJS で表示したい場合、各都市の `tileset.json` を個別に追加する代わりに、**複合 tileset.json API** を利用すると 1 つの URL で対象都市の 3D Tiles を一度に表示できます。これは [3D Tiles 仕様の external tileset 参照](https://github.com/CesiumGS/3d-tiles/tree/main/specification#external-tilesets) を使い、子タイルの `content.uri` で各都市の `tileset.json` を間接的に指す `tileset.json` を動的生成する仕組みです。

```
GET /datacatalog/3dtiles/{spec}/tileset.json
```

例:

```
https://api.plateauview.mlit.go.jp/datacatalog/3dtiles/all-bldg-lod1-2025/tileset.json
```

#### `{spec}` の書式

```
<area>-<type>-<lod>[-interior][-<texture>]-<year>
```

| セグメント | 値 | 意味 |
| --- | --- | --- |
| `area` | `all` | 全国 |
| | 2 桁数字 | 都道府県コード（例: `13` = 東京都） |
| | 5 桁数字 | 市区町村コード（区がある場合は区コード、無ければ市コード） |
| `type` | `bldg` / `tran` / `dem` など | データセットの種別コード |
| `lod` | `lod<N>` | LOD が `<N>` と完全一致するデータのみ |
| | `maxlod<N>` | LOD が `<N>` 以下で、各エリアごとに利用可能な最高 LOD を採用 |
| `interior` | （省略） | 屋外モデル（CityGML 3.0 以前のデータも含む） |
| | `interior` | CityGML 3.0 の屋内モデルのみ |
| `texture` | （省略） | テクスチャありを優先、無ければテクスチャなしを採用 |
| | `texture` | テクスチャありのデータのみ |
| | `notexture` | テクスチャなしのデータのみ |
| `year` | 4 桁の西暦 | その整備年度のデータのみを採用 |
| | `latest` | 整備年度のフィルタを行わず、各自治体について利用可能な最新整備年度のデータを採用 |

例:

| URL | 意味 |
| --- | --- |
| `all-bldg-lod1-2025` | 全国の建築物モデル LOD1（2025年度整備） |
| `all-bldg-maxlod2-2025` | 全国の建築物モデル、各都市で LOD2 まで取れるなら LOD2、無ければ LOD1 |
| `13-bldg-lod2-texture-2025` | 東京都の建築物モデル LOD2、テクスチャあり限定 |
| `13101-bldg-lod2-2025` | 千代田区の建築物モデル LOD2 |
| `all-bldg-lod2-latest` | 全国の建築物モデル LOD2、各自治体ごとに最新整備年度を採用 |

#### CesiumJS での利用例

```javascript
const tileset = await Cesium.Cesium3DTileset.fromUrl(
  "https://api.plateauview.mlit.go.jp/datacatalog/3dtiles/all-bldg-lod1-2025/tileset.json",
);
viewer.scene.primitives.add(tileset);
```

:::caution
- 子の `tileset.json` 自体は別ホスト（PLATEAU CMS）から配信されます。CesiumJS は自動でクロスオリジン取得を行いますが、ネットワーク環境によってはまとめて読み込む際に時間がかかる場合があります。
- API は試験運用中であり、URL の書式やレスポンスは予告なく変更されることがあります。
:::

#### HTTP キャッシュ

`/datacatalog/3dtiles/{spec}/tileset.json`、`/datacatalog/mvt/{spec}/tilejson.json`、`/datacatalog/citygml/{spec}/citygml.zip` の各エンドポイントは、レスポンス内容（CityGML リダイレクトは転送先 URL）から導出した **弱 ETag** (`W/"..."`) と `Cache-Control: no-cache, must-revalidate` を返します。クライアントが `If-None-Match` ヘッダを付けて再リクエストすれば、内容が変わっていなければ `304 Not Modified` で短絡し、ボディは送信されません。CMS 側でデータが差し替わると ETag も変わるため、`-latest` URL を使っていても安全に最新データへ追従できます。

### 4.3. 自治体単位の MVT TileJSON（MapLibre などから利用）

MVT データセットを MapLibre GL JS / Mapbox GL JS のような TileJSON ベースのクライアントから扱うために、自治体単位の **TileJSON 3.0** を動的生成するエンドポイントを提供しています。

```
GET /datacatalog/mvt/{spec}/tilejson.json
```

`{spec}` の書式：

```
<cityCode>-<type>[-lod<N>][-interior]-<year>
```

| セグメント | 値 | 意味 |
| --- | --- | --- |
| `cityCode` | 5 桁数字 | 市区町村コード（区がある場合は区コード、無ければ市コード） |
| `type` | `luse` / `fld` など | データセットの種別コード |
| `lod<N>` | （省略） | LOD が指定されていないデータセットを採用 |
| | `lod<N>` | LOD が `<N>` のデータセットを採用 |
| `interior` | （省略） | 屋外モデル |
| | `interior` | CityGML 3.0 の屋内モデルのみ |
| `year` | 4 桁の西暦 | その整備年度のデータのみを採用 |
| | `latest` | 利用可能な最新整備年度のデータを採用 |

例:

| URL | 意味 |
| --- | --- |
| `13101-luse-2025` | 千代田区の土地利用 MVT（2025年度整備） |
| `13101-luse-latest` | 千代田区の土地利用 MVT、最新整備年度 |
| `13101-fld-lod1-2025` | 千代田区の洪水浸水想定区域 MVT（LOD1） |
| `13101-bldg-lod3-interior-2025` | 千代田区の建築物モデル LOD3、屋内モデルのみ |

[シンプル API](#41-シンプル-api) のレスポンスでも、各 MVT 行の `composite_url` フィールドにこの TileJSON URL が入ります。

### 4.4. GraphQL API

[GraphQL](https://graphql.org/) API では、必要な型・フィールドだけを 1 リクエストで取得できます。エンドポイント:

```
https://api.plateauview.mlit.go.jp/datacatalog/graphql
```

ブラウザで開くと **GraphiQL** が起動し、対話的にクエリを試せます。型定義の一覧は [GraphQL スキーマリファレンス](/api/graphql/schema/)、サンプル付きの埋め込みは [プレイグラウンド](/api/graphql/playground/) を参照してください。

クエリ例（札幌市のデータ一覧）:

```graphql
query {
  area(code: "01100") {
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

```sh
curl -X POST -H 'Content-Type: application/json' \
  -d '{"query":"{ area(code: \"01100\") { datasets { name items { url } } } }"}' \
  https://api.plateauview.mlit.go.jp/datacatalog/graphql
```

:::caution
GraphQL のスキーマやレスポンスは予告なく変更されることがあります。複雑すぎるクエリは動作速度低下防止のため制限されます。レスポンスは数 MB 以上になり得ます。
:::
