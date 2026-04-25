---
title: PLATEAU-Ortho
description: 航空写真オルソ画像タイルデータの配信仕様と利用方法
---

## 1. PLATEAU-Ortho の概要

Project PLATEAU では、航空写真測量によって作成したオルソ画像をタイル化し、PLATEAU-Ortho として配信を行っています。本チュートリアルでは、タイル化技術および PLATEAU-Ortho の利用方法について解説します。

### 1.1. オルソ画像タイルについて

オルソ画像タイルとは、大きなオルソ画像をタイル状に分割したものです。

オルソ画像をタイル化する理由は、現状のインターネットやパソコン、スマホの能力では、大きな画像が重たすぎて、スムーズな配信・表示ができないからです。

PLATEAU VIEW ではオルソ航空写真をあらかじめオルソ画像タイルにしてあるので、どこまでもスクロールしながら綺麗なオルソ画像を楽しむことができるようになっています。PLATEAU VIEW はサーバーから日本全部の画像をダウンロードしているのではなく、その時の表示範囲に必要な画像タイルを、ほんの何十枚かだけ取ってきています。

画像タイル 1 枚のサイズは 256x256 ピクセルの正方形で、非常に小さく軽く作られているので、新しいタイルをどんどんダウンロードしても、表示が重たくなることはありません。

### 1.2. PLATEAU-Ortho の構成

PLATEAU VIEW で見ることのできるオルソ画像タイルは 2 種類あります。

- Project PLATEAU で新たに整備した PLATEAU-Ortho
- 国土地理院が整備した日本全国をカバーする地理院タイル全国最新写真（シームレス）

PLATEAU-Ortho の特徴は、国土地理院が整備している [地理院タイル](https://maps.gsi.go.jp/development/siyou.html) をベースにしつつ、新たに取得したオルソ航空写真を組み合わせ、精度を高めているところです。これにより、全体のカバー率を維持しつつ、ユースケース等に必要な範囲で高精度の地形テクスチャを提供することが可能になっています。また、地理院タイルと比べて新しいデータを利用可能です。

Project PLATEAU が新たに取得したオルソ航空写真は [G空間情報センター](https://www.geospatial.jp/ckan/dataset/plateau) から GeoTIFF 形式で入手可能です。

Project PLATEAU で配布しているデータの利用許諾については、クリエイティブ・コモンズ・ライセンスの表示 4.0 国際等に準拠していますので、無償かつ商用も含めた利用が可能です。詳しくは Project PLATEAU [サイトポリシー](https://www.mlit.go.jp/plateau/site-policy/) をご確認ください。

## 2. 配信 URL

:::caution
本サービスはあくまで試験的な運用であるため、提供期間やサービスレベルについては保証できないことをご了承ください。
:::

### 2.1. PLATEAU-Ortho（XYZ タイル）

2023 年度に作成されたデータが利用可能です。

```
https://api.plateauview.mlit.go.jp/tiles/plateau-ortho-2023/{z}/{x}/{y}.png
```

ズームレベルは 10〜19 に対応しています。

#### 整備地域（XYZ タイル）

| 地域コード | 都道府県 | 市町村 | データ提供年度 |
|---|---|---|---|
| 01100 | 北海道 | 札幌市 | 2020 |
| 03201 | 岩手県 | 盛岡市 | 2024 |
| 03202 | 岩手県 | 宮古市 | 2024 |
| 04100 | 宮城県 | 仙台市 | 2022 |
| 05204 | 秋田県 | 大館市 | 2024 |
| 07201 | 福島県 | 福島市 | 2024 |
| 07203 | 福島県 | 郡山市 | 2020 |
| 07204 | 福島県 | いわき市 | 2020 |
| 07205 | 福島県 | 白河市 | 2020 |
| 07209 | 福島県 | 相馬市 | 2023 |
| 09201 | 栃木県 | 宇都宮市 | 2020 |
| 10201 | 群馬県 | 前橋市 | 2023 |
| 10207 | 群馬県 | 館林市 | 2020 |
| 11202 | 埼玉県 | 熊谷市 | 2024 |
| 11203 | 埼玉県 | 川口市 | 2024 |
| 11208 | 埼玉県 | 所沢市 | 2024 |
| 11210 | 埼玉県 | 加須市 | 2023 |
| 11214 | 埼玉県 | 春日部市 | 2023 |
| 11217 | 埼玉県 | 鴻巣市 | 2024 |
| 11222 | 埼玉県 | 越谷市 | 2023 |
| 11223 | 埼玉県 | 蕨市 | 2024 |
| 11228 | 埼玉県 | 志木市 | 2024 |
| 11230 | 埼玉県 | 新座市 | 2024 |
| 11232 | 埼玉県 | 久喜市 | 2023 |
| 11234 | 埼玉県 | 八潮市 | 2023 |
| 11235 | 埼玉県 | 富士見市 | 2024 |
| 11237 | 埼玉県 | 三郷市 | 2024 |
| 11238 | 埼玉県 | 蓮田市 | 2022 |
| 11240 | 埼玉県 | 幸手市 | 2024 |
| 11241 | 埼玉県 | 鶴ヶ島市 | 2024 |
| 11243 | 埼玉県 | 吉川市 | 2023 |
| 11246 | 埼玉県 | 白岡市 | 2023 |
| 11301 | 埼玉県 | 伊奈町 | 2024 |
| 11324 | 埼玉県 | 三芳町 | 2024 |
| 11385 | 埼玉県 | 上里町 | 2024 |
| 11442 | 埼玉県 | 宮代市 | 2023 |
| 11464 | 埼玉県 | 杉戸市 | 2023 |
| 11465 | 埼玉県 | 松伏町 | 2023 |
| 12210 | 千葉県 | 茂原市 | 2022 |
| 12217 | 千葉県 | 柏市 | 2020 |
| 13100 | 東京都 | 23区 | 2023 |
| 13213 | 東京都 | 東村山市 | 2020 |
| 13219 | 東京都 | 狛江市 | 2023 |
| 13229 | 東京都 | 西東京市 | 2022 |
| 14100 | 神奈川県 | 横浜市 | 2023 |
| 14130 | 神奈川県 | 川崎市 | 2022 |
| 14204 | 神奈川県 | 鎌倉市 | 2024 |
| 15202 | 新潟県 | 長岡市 | 2023 |
| 15222 | 新潟県 | 上越市 | 2023 |
| 17206 | 石川県 | 加賀市 | 2022 |
| 19201 | 山梨県 | 甲府市 | 2022 |
| 20202 | 長野県 | 松本市 | 2020 |
| 20204 | 長野県 | 岡谷市 | 2020 |
| 20206 | 長野県 | 諏訪市 | 2023 |
| 20209 | 長野県 | 伊那市 | 2020 |
| 20214 | 長野県 | 茅野市 | 2022 |
| 20220 | 長野県 | 安曇野市 | 2024 |
| 21202 | 岐阜県 | 大垣市 | 2024 |
| 22203 | 静岡県 | 沼津市 | 2020 |
| 22213 | 静岡県 | 掛川市 | 2020 |
| 23100 | 愛知県 | 名古屋市 | 2020 |
| 23201 | 愛知県 | 豊橋市 | 2023 |
| 23206 | 愛知県 | 春日井市 | 2023 |
| 23207 | 愛知県 | 豊川市 | 2022 |
| 23208 | 愛知県 | 津島市 | 2020 |
| 24202 | 三重県 | 四日市市 | 2023 |
| 26100 | 京都府 | 京都市 | 2022 |
| 27100 | 大阪府 | 大阪市 | 2024 |
| 27203 | 大阪府 | 豊中市 | 2020 |
| 27204 | 大阪府 | 池田市 | 2020 |
| 28201 | 兵庫県 | 姫路市 | 2023 |
| 28210 | 兵庫県 | 加古川市 | 2020 |
| 28225 | 兵庫県 | 朝来市 | 2022 |
| 31202 | 鳥取県 | 米子市 | 2024 |
| 31384 | 鳥取県 | 日吉津村 | 2023 |
| 32204 | 島根県 | 益田市 | 2024 |
| 33211 | 岡山県 | 備前市 | 2023 |
| 34100 | 広島県 | 広島市 | 2022 |
| 34304 | 広島県 | 海田町 | 2024 |
| 36201 | 徳島県 | 徳島市 | 2023 |
| 37201 | 香川県 | 高松市 | 2022 |
| 38201 | 愛媛県 | 松山市 | 2020 |
| 40202 | 福岡県 | 大牟田市 | 2023 |
| 40203 | 福岡県 | 久留米市 | 2020 |
| 40225 | 福岡県 | うきは市 | 2023 |
| 40447 | 福岡県 | 筑前町 | 2023 |
| 42323 | 長崎県 | 波佐見町 | 2024 |
| 43100 | 熊本県 | 熊本市 | 2020 |
| 43206 | 熊本県 | 玉名市 | 2020 |
| 43443 | 熊本県 | 益城町 | 2023 |
| 47201 | 沖縄県 | 那覇市 | 2020 |

### 2.2. PLATEAU-Ortho（OGC WMS）

2020 年度に作成されたデータが WMS で利用可能です。

**サービスアドレス**:

```
https://plateauortho.geospatial.jp/mapproxy/service
```

**WMS バージョン**: 1.1.1 または 1.3.0 に準拠。

**サポートする WMS リクエスト**: `GetCapabilities`, `GetMap`

**座標系**: EPSG:4326、EPSG:3857、EPSG:900913、CRS:84

**レイヤー**:

| レイヤー名 | 内容 |
|:---:|:---:|
| `plateau` | PLATEAU-Ortho |
| `photo` | 国土地理院の写真（ベースマップ） |
| `plateau_photo` | `plateau` と `photo` の重ね合わせ |

**リクエスト例**:

- GetCapabilities

  ```
  https://plateauortho.geospatial.jp/mapproxy/service?REQUEST=GetCapabilities
  ```

- GetMap (EPSG:3857)

  ```
  https://plateauortho.geospatial.jp/mapproxy/service?SERVICE=WMS&VERSION=1.3.0&REQUEST=GetMap&BBOX=15528612.30738627352,4234060.365724110045,15581327.2790426109,4277115.012342031114&CRS=EPSG:3857&WIDTH=1075&HEIGHT=878&LAYERS=plateau_photo&STYLES=&FORMAT=image/png&TRANSPARENT=TRUE
  ```

- GetMap (EPSG:4326)

  ```
  https://plateauortho.geospatial.jp/mapproxy/service?SERVICE=WMS&VERSION=1.3.0&REQUEST=GetMap&BBOX=35.62364808927861759,139.853018468884585,35.67783063591231496,139.9193581472800645&CRS=EPSG:4326&WIDTH=1075&HEIGHT=878&LAYERS=plateau_photo&STYLES=&FORMAT=image/png&TRANSPARENT=TRUE
  ```

**ズームレベル**: ズームレベル 19 までの拡大が可能で、約 30cm の解像度に相当します。

| ズームレベル | 画像解像度 |
|:---:|:---:|
| 10 | 153m |
| 11 | 76m |
| 12 | 38m |
| 13 | 19m |
| 14 | 10m |
| 15 | 5m |
| 16 | 2m |
| 17 | 1.2m |
| 18 | 0.6m |
| 19 | 0.3m |

## 3. PLATEAU-Ortho の利用方法

本節では、PLATEAU-Ortho（XYZ タイル）を CesiumJS、TerriaJS、Leaflet、QGIS で利用する方法について解説します。

### 3.1. CesiumJS で利用する方法

CesiumJS は一般的に使われている 3 次元地図を表示するための JavaScript ライブラリです。

CesiumJS でオルソ画像タイルを利用するには、地形モデルもあわせて使用することをおすすめします。ここでは、地形データとして PLATEAU-Terrain を利用し、PLATEAU-Ortho を表示するためのサンプルコードを示します。

PLATEAU-Terrain については [PLATEAU-Terrain](/datasets/terrain/) を、建物モデルは [PLATEAU-3DTiles/MVT](/datasets/3d-tiles/) を参照してください。

```html
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <title>PLATEAU-3DTiles/MVT、PLATEAU-Ortho、PLATEAU-Terrain を Cesium で表示</title>
  <script src="https://cesium.com/downloads/cesiumjs/releases/1.117/Build/Cesium/Cesium.js"></script>
  <link href="https://cesium.com/downloads/cesiumjs/releases/1.117/Build/Cesium/Widgets/widgets.css" rel="stylesheet">
  <style>
    #cesiumContainer { position: absolute; top: 0; left: 0; height: 100%; width: 100%; margin: 0; }
    html, body { height: 100%; margin: 0; padding: 0; overflow: hidden; }
  </style>
</head>
<body>
  <div id="cesiumContainer"></div>
  <script>
    Cesium.Ion.defaultAccessToken = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiJlNjk0MTM4NC1lMWI0LTQxNTgtYjcxZS01ZWJhMGJlMTE1MWQiLCJpZCI6MTQ5ODk3LCJpYXQiOjE3MTUxNTEyODZ9.2aUmEQ2-fDsjf-XeC6-hZpwkgwLse3yXoXF4xTOvPAY";
    const viewer = new Cesium.Viewer("cesiumContainer", {});

    viewer.scene.setTerrain(
      new Cesium.Terrain(Cesium.CesiumTerrainProvider.fromIonAssetId(2488101)),
    );

    viewer.scene.imageryLayers.addImageryProvider(
      new Cesium.UrlTemplateImageryProvider({
        url: 'https://api.plateauview.mlit.go.jp/tiles/plateau-ortho-2023/{z}/{x}/{y}.png',
        maximumLevel: 19
      })
    );

    Cesium.Cesium3DTileset.fromUrl(
      'https://assets.cms.plateau.reearth.io/assets/0e/e5948a-e95c-4e31-be85-1f8c066ed996/13101_chiyoda-ku_pref_2023_citygml_1_op_bldg_3dtiles_13101_chiyoda-ku_lod1/tileset.json'
    ).then((tileset) => viewer.scene.primitives.add(tileset));

    viewer.camera.setView({
      destination: Cesium.Cartesian3.fromDegrees(139.76, 35.68, 5000.0)
    });
  </script>
</body>
</html>
```

### 3.2. TerriaJS で利用する方法

TerriaJS のカタログファイルに以下のように記述することで、PLATEAU-Ortho を TerriaJS のレイヤとして組み込むことができます。

```json
{
  "type": "open-street-map",
  "id": "/basemap//オルソ画像タイル/imagery",
  "name": "航空写真",
  "url": "https://api.plateauview.mlit.go.jp/tiles/plateau-ortho-2023/",
  "fileExtension": "png",
  "attribution": ""
}
```

Project PLATEAU では [PLATEAU VIEW 1.1](https://github.com/Project-PLATEAU/PLATEAU-VIEW-1.1) およびそのための [カタログファイルジェネレータ](https://github.com/Project-PLATEAU/plateau-catalog-generator) のソーススクリプトを公開していますので、そちらも参照してください。

### 3.3. Leaflet で利用する方法

以下の HTML は Leaflet でオルソ画像タイルを表示するサンプルです。`ortho_test.html` などの名前で PC に保存して、ブラウザで開いてみてください。

```html
<html lang="ja">
<head>
  <title>Leaflet で PLATEAU-Ortho を表示する</title>
  <meta charset="utf-8" />
  <link rel="stylesheet" href="https://unpkg.com/leaflet@1.7.1/dist/leaflet.css" />
  <script src="https://unpkg.com/leaflet@1.7.1/dist/leaflet.js"></script>
  <script>
    document.addEventListener("DOMContentLoaded", () => {
      const map = L.map('map').setView([35.68, 139.76], 14);
      map.options.minZoom = 10;
      map.options.maxZoom = 18;
      L.tileLayer('https://api.plateauview.mlit.go.jp/tiles/plateau-ortho-2023/{z}/{x}/{y}.png', {
        attribution: '<a href="https://www.mlit.go.jp/plateau/">国土交通省 Project PLATEAU</a>'
      }).addTo(map);
    });
  </script>
</head>
<body>
  <div id="map" style="width: 80%; height: 600px; border: solid 1px"></div>
</body>
</html>
```

### 3.4. QGIS で利用する方法

[QGIS](https://qgis.org/ja/site/) は Windows や Mac で使える、オープンソースの GIS ソフトです。QGIS では GIS データを表示するときの背景図として、PLATEAU-Ortho を使うことができます。

1. QGIS のブラウザタブにある「XYZ Tiles」を右クリックして、「新規接続」を選びます

   ![QGIS XYZ メニュー](../../../assets/datasets/ortho/ortho_qgis_xyz.png)

2. 下図のウインドウが出てきますので、名前、URL、最大ズームレベルを記入してください
    - URL 欄に `https://api.plateauview.mlit.go.jp/tiles/plateau-ortho-2023/{z}/{x}/{y}.png` を入力
    - 最大ズームレベルは `19`
    - 「OK」を押します

   ![QGIS XYZ ダイアログ](../../../assets/datasets/ortho/ortho_qgis_xyz_dialog.png)

3. ブラウザウインドウに「PLATEAU-Ortho 画像タイル」が追加されます

4. 地図にオルソ画像タイルを組み込んで表示。最初に「OpenStreetMap」をつかんでレイヤタブに入れ、次に「PLATEAU-Ortho 画像タイル」をつかんでレイヤタブに入れます

   ![QGIS ドラッグ&ドロップ](../../../assets/datasets/ortho/ortho_qgis_drag.png)

5. 拡大すると、PLATEAU-Ortho の画像タイルが表示されます

   ![QGIS マップ](../../../assets/datasets/ortho/orth_gqis_map.png)

## 4. オルソ画像タイルを作成する方法

オルソ画像タイルを作るには専用のソフトウェアを利用します。

Project PLATEAU では、大量のオルソ画像のタイル化を効率的に行うために、FME という市販のソフトウェアを利用していますが、オープンソースやフリーソフトでも、オルソ画像タイルを作成することができます。

例えば、以下のウェブサイトでは QGIS というオープンソースの GIS ソフトを利用したタイル生成の方法が紹介されています。

- [「GIS 実習オープン教材」タイル地図入門（ラスタ）](https://gis-oer.github.io/gitbook/book/materials/web_gis/rastertile/rastertile.html)

オルソ画像タイルを作成する手順は以下の 2 ステップです。

1. **オルソ画像の準備**: オルソ画像の材料は航空写真や衛星画像です。普通は航空測量会社やリモートセンシング技術センターなどから購入することになりますが、小さな範囲であれば、ドローンを使って自前のオルソ航空写真を作成することができます。なおドローン撮影には一般的に関係部署の許可が必要です。
2. **オルソ画像のタイル化**: オルソ画像を専用ソフトに読み込ませて、オルソ画像タイルを保存します。
