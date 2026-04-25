---
title: PLATEAU-3DTiles / MVT
description: 3D Tiles および MVT 形式での建築物モデル等の配信仕様と利用方法
---

## 1. PLATEAU-3DTiles / MVT の概要

Project PLATEAU では、CityGML 形式で作成された 3D 都市モデルのデータを 3D Tiles 1.0 および MVT（Mapbox Vector Tiles）形式に変換し、それぞれ配信を行っています。

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

    // 東京都千代田区の建築物モデル（3D Tiles）
    Cesium.Cesium3DTileset.fromUrl(
      'https://api.plateauview.mlit.go.jp/datacatalog/3dtiles/13101-bldg-lod2-2025/tileset.json'
    ).then((tileset) => {
      viewer.scene.primitives.add(tileset);
    });

    // 東京都の土地利用モデル（MVT）
    const yourMvt = new CesiumMVTImageryProvider.CesiumMVTImageryProvider({
      urlTemplate: "https://assets.cms.plateau.reearth.io/assets/4c/efcbfe-f523-4a59-92f8-f6af80882333/13_tokyo_pref_2023_citygml_1_op_luse_mvt/{z}/{x}/{y}.mvt",
      layerName: "luse",
      style: feature => ({ fillStyle: "white" }),
    });
    viewer.scene.imageryLayers.addImageryProvider(yourMvt);

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

主要なフィールド: `name`, `pref` / `pref_code`, `city` / `city_code`, `ward` / `ward_code`, `type` / `type_en`, `url`, `composite_url`（3D Tiles のみ。後述の複合 tileset.json への直リンク）, `layers`（MVT のみ）, `year`, `registration_year`, `spec`, `format`（`3D Tiles` または `MVT`）, `lod`, `texture`。型と意味の一覧は [REST API リファレンス](/api/rest/operations/datacatalogplateau-datasets/) を参照してください。

レスポンスにはさらに `composite_tilesets` 配列が含まれます。これは全国（`all-...`）と都道府県別（`13-...` など）の複合 tileset.json を実データから派生して列挙したリストで、CesiumJS で広域をまとめて表示したい場合の入口として利用できます。

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
<area>-<type>-<lod>[-<texture>]-<year>
```

| セグメント | 値 | 意味 |
| --- | --- | --- |
| `area` | `all` | 全国 |
| | 2 桁数字 | 都道府県コード（例: `13` = 東京都） |
| | 5 桁数字 | 市区町村コード（区がある場合は区コード、無ければ市コード） |
| `type` | `bldg` / `tran` / `dem` など | データセットの種別コード |
| `lod` | `lod<N>` | LOD が `<N>` と完全一致するデータのみ |
| | `maxlod<N>` | LOD が `<N>` 以下で、各エリアごとに利用可能な最高 LOD を採用 |
| `texture` | （省略） | テクスチャありを優先、無ければテクスチャなしを採用 |
| | `texture` | テクスチャありのデータのみ |
| | `notexture` | テクスチャなしのデータのみ |
| `year` | 4 桁の西暦 | データの整備年度 |

例:

| URL | 意味 |
| --- | --- |
| `all-bldg-lod1-2025` | 全国の建築物モデル LOD1（2025年度整備） |
| `all-bldg-maxlod2-2025` | 全国の建築物モデル、各都市で LOD2 まで取れるなら LOD2、無ければ LOD1 |
| `13-bldg-lod2-texture-2025` | 東京都の建築物モデル LOD2、テクスチャあり限定 |
| `13101-bldg-lod2-2025` | 千代田区の建築物モデル LOD2 |

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

### 4.3. GraphQL API

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
