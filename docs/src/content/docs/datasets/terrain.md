---
title: PLATEAU-Terrain
description: 日本全国の地形データ配信サービスの利用方法
---

## 1. PLATEAU-Terrain の概要

Project PLATEAU では、日本全国の地形データを Cesium などの 3D 地図エンジンから直接利用できるタイル配信サービス「PLATEAU-Terrain」を提供しています。

国土地理院の基盤地図情報数値標高モデル（DEM）をはじめとした各種データソースから生成された標高値に、日本のジオイドモデル（[GSIGEO2011](https://www.gsi.go.jp/buturisokuchi/grageo_geoidseika.html) など）を合成し、**楕円体高（ellipsoidal height）** に変換した地形を配信しています。これにより、3D Tiles などのジオコード済みデータと垂直方向のずれが発生しないようになっています。

本チュートリアルでは、PLATEAU-Terrain の利用方法について解説します。

:::note[現在の提供状況]
現在配信しているのは、Cesium 向けの **quantized-mesh 形式**の地形タイル（`/terrain/`）です。これは従来 Cesium ion 上でホストしていた PLATEAU-Terrain（[5 章](#5-plateau-terrain-cesium-ion)）をオブジェクトストレージにミラーし、そのまま配信しているものです。配信される標高は楕円体高に変換済みですが、**ジオイドモデルはデータ生成時に固定**されており、リクエスト時に切り替えることはできません。

リクエスト時のジオイドモデル切り替え（`?geoid=`）や、MapLibre / Mapbox GL 向けの raster-dem エンドポイント（`/terrarium/`・`/mapbox/`）は、新しい DEM ベースの配信パイプラインで開発を進めていますが、**現在調整中で、まだリリースされていません**（[1.3](#13-今後提供予定の機能調整中)）。
:::

### 1.1. 高さの基準（楕円体高）

3D 地図エンジンは一般に WGS84 楕円体（GPS と同じ基準面）を 3 次元空間の基準にしているのに対し、国土地理院の DEM や日常的な「標高」は **正標高（orthometric height、平均海面からの高さ）** で表現されています。両者の差はジオイド高 N と呼ばれ、日本付近では概ね **+30〜+45 m** あります。この差を補正しないと、3D Tiles などのデータと地形が垂直方向にずれてしまいます。

```
ellipsoidal height = orthometric height + geoid height (N)
```

PLATEAU-Terrain では、ジオイド高 N を加算した楕円体高を配信しています。現在配信中の `/terrain/`（quantized-mesh ミラー）では、この補正がデータ生成時に済んでおり、固定のジオイドモデルが適用されています。

### 1.2. ズームレベル

最大ズームは `/terrain/layer.json` の `maxzoom` で確認できます。

### 1.3. 今後提供予定の機能（調整中）

以下の機能は新しい DEM ベースの配信パイプラインで開発を進めていますが、調整中のためまだリリースされていません。正式公開までは [2 章](#2-配信-url)の quantized-mesh 配信をご利用ください。

- **ジオイドモデルの切り替え**：`?geoid=` クエリパラメータで、用途に応じてジオイドモデルを切り替え。提供予定のモデルは次のとおりです。

  | `geoid=` | 内容 | 適用範囲 |
  | --- | --- | --- |
  | `gsigeo2011`（既定） | 国土地理院「日本のジオイド 2011」(Ver.2.2) | 日本陸域 |
  | `jpgeo2024` | 国土地理院「日本のジオイド 2024」 | 日本陸域 + 周辺海域 |
  | `jpgeo2024-hrefconv` | JPGEO2024 + Hrefconv 補正 | 日本陸域のみ |
  | `none` | ジオイド補正なし（正標高のまま） | グローバル |

- **MapLibre / Mapbox GL 向け raster-dem エンドポイント**：同じ標高データを MapLibre / Mapbox GL からも利用できるよう、`/terrarium/`（[Mapzen Terrarium](https://github.com/tilezen/joerd/blob/master/docs/formats.md#terrarium)）と `/mapbox/`（[Mapbox Terrain-RGB v1](https://docs.mapbox.com/data/tilesets/reference/mapbox-terrain-dem-v1/)）の各エンドポイントを PNG / WebP / AVIF 形式で配信。
- **高ズームのアップサンプリング**：DEM 元データの最大ズームを超えるズームが要求された場合に、親タイルを bilinear でアップサンプリングして返却。

## 2. 配信 URL

:::caution
本サービスはあくまで試験的な運用であるため、提供期間やサービスレベルについては保証できないことをご了承ください。またデータの内容は予告なく更新されることがあります。
:::

ベース URL:

```
https://tile.plateauview.mlit.go.jp
```

### 2.1. Cesium 用 quantized-mesh terrain

```
https://tile.plateauview.mlit.go.jp/terrain/layer.json
https://tile.plateauview.mlit.go.jp/terrain/{z}/{x}/{y}.terrain
```

[Cesium quantized-mesh-1.0](https://github.com/CesiumGS/quantized-mesh) 形式（TMS Geodetic、`octvertexnormals` 拡張付き）の地形タイルです。CesiumJS の `CesiumTerrainProvider.fromUrl` に `layer.json` の URL（または `/terrain`）を渡すだけで利用できます。`requestVertexNormals: true` を指定すると法線付きで読み込めます。

:::note
現在の `/terrain/` は、固定のジオイドモデルを適用済みの quantized-mesh ミラーです。`?geoid=` を付与してもジオイドモデルは切り替わりません（[1.3](#13-今後提供予定の機能調整中)）。
:::

### 2.2. 配信中のタイル一覧（カタログ API）

`/tiles/catalog.json` で、現在配信中の全タイルソース（オルソ画像と地形を含む）の一覧をプログラムから取得できます。地形は次のエントリとして含まれます。

```
https://tile.plateauview.mlit.go.jp/tiles/catalog.json
```

| `name` | `urls` のキー | URL |
| --- | --- | --- |
| `terrain` | `quantized-mesh` | `/terrain/layer.json` |

レスポンス例（地形部分の抜粋）:

```json
{
  "tiles": [
    {
      "name": "terrain",
      "description": "Cesium quantized-mesh terrain (ellipsoidal heights, Japan coverage)",
      "urls": {
        "quantized-mesh": "https://tile.plateauview.mlit.go.jp/terrain/layer.json"
      }
    }
  ]
}
```

## 3. 利用例

### 3.1. CesiumJS

`Cesium.Ion` のトークンは不要です。`CesiumTerrainProvider.fromUrl` に `/terrain/` の URL を渡すだけで動作します。

```html
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <title>PLATEAU-Terrain を Cesium で表示</title>
  <script src="https://cesium.com/downloads/cesiumjs/releases/1.127/Build/Cesium/Cesium.js"></script>
  <link href="https://cesium.com/downloads/cesiumjs/releases/1.127/Build/Cesium/Widgets/widgets.css" rel="stylesheet">
  <style>
    #cesiumContainer { position: absolute; inset: 0; margin: 0; }
    html, body { height: 100%; margin: 0; }
  </style>
</head>
<body>
  <div id="cesiumContainer"></div>
  <script>
    const viewer = new Cesium.Viewer("cesiumContainer", { baseLayer: false });

    // PLATEAU-Terrain（quantized-mesh、ellipsoidal heights）
    Cesium.CesiumTerrainProvider.fromUrl(
      "https://tile.plateauview.mlit.go.jp/terrain",
      { requestVertexNormals: true },
    ).then((provider) => {
      viewer.terrainProvider = provider;
    });

    // 地理院タイル（標準地図）をドレープ
    viewer.imageryLayers.addImageryProvider(
      new Cesium.UrlTemplateImageryProvider({
        url: "https://cyberjapandata.gsi.go.jp/xyz/std/{z}/{x}/{y}.png",
        maximumLevel: 18,
      }),
    );

    viewer.camera.setView({
      destination: Cesium.Cartesian3.fromDegrees(138.73, 35.36, 8000),
      orientation: { pitch: Cesium.Math.toRadians(-30) },
    });
  </script>
</body>
</html>
```

### 3.2. MapLibre GL JS

MapLibre / Mapbox GL から利用するための raster-dem エンドポイント（`/terrarium/`・`/mapbox/`）は現在調整中で、まだリリースされていません（[1.3](#13-今後提供予定の機能調整中)）。正式公開後に利用方法を追記します。

### 3.3. プレビュー

ブラウザで以下にアクセスすると、Cesium 上で地形表現を試せるプレビューが利用できます。

- Cesium プレビュー: <https://tile.plateauview.mlit.go.jp/terrain-viewer>

## 4. 帰属表示

PLATEAU-Terrain を利用する場合は、地図画面上に下記のいずれかの帰属を必ず表示してください。

```
PLATEAU | Mapterhorn | 国土地理院
```

`/terrain/layer.json` のレスポンスにも同等の `attribution` 文字列が含まれます。

## 5. PLATEAU-Terrain (Cesium ion)

2024 年度以前から提供している、Cesium ion でホストされた `terraindb` 形式の地形データです。新たに 2 章で説明したタイル配信サービスを利用される場合はそちらが推奨ですが、Cesium ion 経由で従来通り利用したい場合は本節の手順を参照してください。

### 5.1. アクセストークンおよびアセット ID

PLATEAU-Terrain (Cesium ion) を Cesium で利用する際は以下のトークンとアセット ID を利用してください。

:::caution
本サービスはあくまで試験的な運用であるため、提供期間やサービスレベルについては保証できないことをご了承ください。またデータの内容は予告なく更新されることがあります。
:::

**トークン**:

```
eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiJiODVhMmQ5OS1hOWZjLTQ3YmYtODlmNi1lNWUwY2MwOGUxYTMiLCJpZCI6MTQ5ODk3LCJpYXQiOjE2ODc5MzQ3NDN9.OG0mc3i7ZxGwHQjlMv3TRjiOvKWpzxglxmJRaUIykTY
```

**アセット ID**:

```
3258112
```

**使用例**:

```js
Cesium.Ion.defaultAccessToken = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiJiODVhMmQ5OS1hOWZjLTQ3YmYtODlmNi1lNWUwY2MwOGUxYTMiLCJpZCI6MTQ5ODk3LCJpYXQiOjE2ODc5MzQ3NDN9.OG0mc3i7ZxGwHQjlMv3TRjiOvKWpzxglxmJRaUIykTY";

viewer.scene.setTerrain(
  new Cesium.Terrain(
    Cesium.CesiumTerrainProvider.fromIonAssetId(3258112),
  ),
);
```

### 5.2. CesiumJS アプリケーションの作成

CesiumJS 上で PLATEAU-Terrain (Cesium ion) を利用するためのサンプルコードを示します。

地形データの配信についてご質問がある方は、PacificSpatialSolutions 株式会社（info@pacificspatial.com）までご連絡ください。

配信された地形データを利用する場合は、「地形データは、測量法に基づく国土地理院長承認（使用）R3JHs 778 を得て使用」とデータの帰属に記載してください。

なお、建物モデルは [PLATEAU-3DTiles](/datasets/3d-tiles/) から配信されている千代田区の建物モデルデータを、ドレープするオルソ写真は [PLATEAU-Ortho](/datasets/ortho/) から配信されている東京都 23 区の航空写真を参照しています。

```html
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <title>PLATEAU-3DTiles/MVT、PLATEAU-Ortho、PLATEAU-Terrain (Cesium ion) を Cesium で表示</title>
  <script src="https://cesium.com/downloads/cesiumjs/releases/1.117/Build/Cesium/Cesium.js"></script>
  <link href="https://cesium.com/downloads/cesiumjs/releases/1.117/Build/Cesium/Widgets/widgets.css" rel="stylesheet">
  <style>
    #cesiumContainer { position: absolute; top: 0; left: 0; height: 100%; width: 100%; margin: 0; overflow: hidden; padding: 0; }
    html, body { height: 100%; margin: 0; padding: 0; overflow: hidden; }
  </style>
</head>
<body>
  <div id="cesiumContainer"></div>
  <script>
    Cesium.Ion.defaultAccessToken = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiJiODVhMmQ5OS1hOWZjLTQ3YmYtODlmNi1lNWUwY2MwOGUxYTMiLCJpZCI6MTQ5ODk3LCJpYXQiOjE2ODc5MzQ3NDN9.OG0mc3i7ZxGwHQjlMv3TRjiOvKWpzxglxmJRaUIykTY";
    const viewer = new Cesium.Viewer("cesiumContainer", {});

    viewer.scene.setTerrain(
      new Cesium.Terrain(Cesium.CesiumTerrainProvider.fromIonAssetId(3258112)),
    );

    viewer.scene.imageryLayers.addImageryProvider(
      new Cesium.UrlTemplateImageryProvider({
        url: 'https://tile.plateauview.mlit.go.jp/tiles/plateau-ortho-2023/{z}/{x}/{y}.png',
        maximumLevel: 19
      })
    );

    Cesium.Cesium3DTileset.fromUrl(
      'https://api.plateauview.mlit.go.jp/datacatalog/3dtiles/13101-bldg-lod1-latest/tileset.json'
    ).then((tileset) => viewer.scene.primitives.add(tileset));

    viewer.camera.setView({
      destination: Cesium.Cartesian3.fromDegrees(139.76, 35.68, 5000.0)
    });
  </script>
</body>
</html>
```

:::note
CesiumJS では、PLATEAU-Terrain 以外に、地形データとして Cesium がデフォルトで配信している Cesium World Terrain を利用することもできます。日本地域における地形データ詳細度は、Cesium World Terrain のほうが PLATEAU-Terrain より劣りますが、世界各地で地形表現を有効にできます。
:::

```js
const viewer = new Cesium.Viewer('cesiumContainer', {
  terrainProvider: Cesium.createWorldTerrain()
});
```

### 5.3. Cesium ion 用地形データの作成

PLATEAU-Terrain (Cesium ion) などのオープンデータとして利用できる地形データを利用することに加え、独自に地形データを作成することも可能です。

本節では、独自に地形データを整備するために必要なデジタル標高モデル（DEM）データの作成方法について説明します。

#### 5.3.1. DEM データの作成

地形データを作成する場合は、地形データのもととなるラスター形式のデジタル標高モデル（DEM）を準備する必要があります。

ここでは、国土地理院が整備する基盤地図情報数値標高モデルを利用し、FME を用いて DEM データを作成する方法について説明します。

なお、DEM データは様々な方法で作成可能であり、必ずしも FME を利用する必要はありません。

##### (1) 作業フォルダと FME ワークスペースの格納

基盤地図情報数値標高モデルから地形モデル用 GeoTIFF 形式 DEM ラスターへの変換を行うため、以下のワークスペースをダウンロードして作業フォルダ内に格納してください。

- [s1_基盤地図情報DEMのGeoTIFF変換.fmw](/fme/s1_基盤地図情報DEMのGeoTIFF変換.fmw)
- [s1_基盤地図情報DEMのGeoTIFF変換_runner.fmw](/fme/s1_基盤地図情報DEMのGeoTIFF変換_runner.fmw)
- [s2_海面つきDEM10B_GeoTIFF作成.fmw](/fme/s2_海面つきDEM10B_GeoTIFF作成.fmw)

これらの FME ワークスペースは、以下の条件で稼働することを確認しています。

- FME 2021.2.2 またはそれ以降のバージョンの FME Desktop
- FME Hub で公開されているカスタムフォーマット [Japanese Fundamental Geospatial Data (FGD) DEM V2](https://hub.safe.com/publishers/pacific-spatial-solutions/formats/japanese-fundamental-geospatial-data-fgd-dem-v2) のダウンロードとインストール
- ダウンロードした `JP_FGD_DEM2.fds` をエクスプローラー上で右クリックしてインストール

##### (2) DEM データのダウンロード

基盤地図情報ダウンロードサイトから対象とする地域の数値標高モデル（DEM）データ（5A, 5B, 5C, 10A, 10B）をすべてダウンロードし、作業フォルダーに保存してください。

基盤地図情報 DEM の各データセット（2 次メッシュ区画単位）の zip ファイルは解凍しないでください。基盤地図情報ダウンロードサイトの「まとめてダウンロード」機能を使って複数のデータセット（zip）をさらにまとめた zip ファイルでダウンロードした場合には、最上位の zip のみ解凍して 2 次メッシュ区画単位のデータセットに展開しておいてください。

DEM の種類（5A, 5B, 5C, 10A, 10B）ごとにサブフォルダーに分ける必要はありません。

##### (3) FME ワークスペースの実行

###### (3-a) 基盤地図情報 DEM データから GeoTIFF 形式 DEM ラスターへの変換

ワークスペース: `s1_基盤地図情報DEMのGeoTIFF変換_runner.fmw`

このワークスペースを実行することにより、作業フォルダー以下（サブフォルダーも含む）に保存したすべての基盤地図情報 DEM データを DEM ラスターに変換し、出力先として指定したフォルダー内の DEM タイプ別サブフォルダー（`dem5a`, `dem5b`, `dem5c`, `dem10a`, `dem10b`）に GeoTIFF 形式で保存します。

![FME runner ダイアログ](../../../assets/datasets/terrain/terrain_fme_runner_dialog.png)

図 1 `s1_基盤地図情報DEMのGeoTIFF変換_runner.fmw` 実行時のダイアログ

| パラメーター名 | 内容 |
| --- | --- |
| FGD DEM Download Folder | 基盤地図情報 DEM データをダウンロードしたルートフォルダー（`downloads` フォルダー） |
| Destination GeoTIFF Root Folder | 変換結果 GeoTIFF ファイルの出力先ルートフォルダー |
| Log Folder | ログファイル出力先フォルダー |
| Maximum Concurrent FME Processes | 同時に実行する FME プロセス数（M, 最大 7） |
| Workspace Runs per FME Process | 1 つの FME プロセスによるワークスペースの実行回数（N） |

このワークスペースは、基盤地図情報 DEM データ（`*.zip`）を 1 ファイルずつ GeoTIFF 形式に変換するための子ワークスペース `s1_基盤地図情報DEMのGeoTIFF変換.fmw` をファイル数分、繰り返し実行します。

###### (3-b) 海域に標高値を与えた DEM ラスター（10B）の作成

ワークスペース: `s2_海面つきDEM10B_GeoTIFF作成.fmw`

`s1_基盤地図情報DEMのGeoTIFF変換_runner.fmw` によって基盤地図情報 DEM から GeoTIFF 形式 DEM ラスターが作成できますが、全国をカバーしている 10B でも海域は Nodata（値がない）であるため、このまま Cesium の地形モデルに変換すると陸域と海域の境界付近で高さが不連続になることがあります。このワークスペースを実行することにより、Nodata のセルに標高値 0 を与えるとともに、若干沖合に範囲を拡大した 10B の DEM ラスターデータセットを作成できます。

![FME 10B ダイアログ](../../../assets/datasets/terrain/terrain_fme_dem10b.png)

図 2 ワークスペース `s2_海面つきDEM10B_GeoTIFF作成.fmw` 実行時のダイアログ

| パラメーター名 | 内容 |
| --- | --- |
| Source GeoTIFF Folder (dem10b) | 変換済みの GeoTIFF 形式 10B DEM ラスター保存先フォルダー |
| Destination GeoTIFF Root Folder | 変換結果 GeoTIFF ファイルの出力先フォルダー |

基盤地図情報 DEM は、10B 以外は全国の陸域をカバーしているわけではありません。標高値の精度は次の順で高くなります。

- 低: `10B < 10A < 5C < 5B < 5A` :高

そのため、Cesium ion の地形モデル（terrain）を作成する場合は、次の順に 1 つ前に作成した地形モデルに次の地形モデルを上乗せしていくことにより、地点ごとに存在する DEM タイプのうち最も精度が高いものの標高値を使用した地形モデルとなります。

- 海域に標高値を与えた `10B → 10A → 5C → 5B → 5A`

#### 5.3.2. 地形データへの変換

ここでは、Cesium ion を利用した DEM データの地形データへの変換方法を説明します。

Cesium ion のサービスを利用することで、データの変換後、地形データの配信も行えるようになります。なお、Cesium ion は有償サービスですが、一定の範囲内であれば無償で利用可能です。

Cesium ion では、ユーザーがアップロードした DEM ファイルを `terraindb` 形式に変換し、配信に利用可能です。`terraindb` 形式への変換は、Cesium ion のクラウドサービスに加え、Cesium のオンプレミス（有償）で提供される変換プログラムでも行えます。日本全国の詳細な地形データを作成する場合は、Cesium のオンプレミスの利用をお勧めします。

##### (1) Cesium ion アカウント開設

まず、[Cesium ion](https://cesium.com/) のアカウントを用意します。データサイズが 50 GB を超える場合、有料の Commercial アカウント以上を利用する必要があります。

##### (2) 作成した DEM のアップロードと変換

FME 等で作成した地形データ作成用の DEM を Cesium ion のアカウントにアップロードして `terraindb` 形式の地形データを準備します。

FME により複数の水平解像度の DEM を用意した場合は、海域に標高値を与えた `10B → 10A → 5C → 5B → 5A` の順にアップロードし、解像度の高い DEM が一番最後にくるようにしてください。

以下では例として、5B / 5A の DEM をアップロードし、地形データを作成する手順を示します。

1. My Asset タブを開き、「Add data」と書かれた青いボタンをクリック

   ![Cesium ion ダイアログ 1](../../../assets/datasets/terrain/terrain_ceisum_ion_dialog_1.png)

2. Add Data ページで、「Add files...」と書かれた青いボタンをクリックし、対象となるファイルを指定

   ![Cesium ion ダイアログ 2](../../../assets/datasets/terrain/terrain_ceisum_ion_dialog_2.png)

3. 対象ファイルがリストに現れたら、Asset name を入力し、「What kind of data is this?」を `Raster Terrain` に指定。最初は Base Terrain として `Mean Sea Level` を指定し、「Upload」ボタンをクリック

   ![Cesium ion ダイアログ 3](../../../assets/datasets/terrain/terrain_ceisum_ion_dialog_3.png)

4. アップロードして地形データとして変換された 5B データを確認したら、再び「Add data」ボタンをクリックして、次の 5A の DEM ファイルを指定

   ![Cesium ion ダイアログ 4](../../../assets/datasets/terrain/terrain_ceisum_ion_dialog_4.png)

5. Base Terrain として、先ほどアップロードして変換したデータを指定

   ![Cesium ion ダイアログ 5](../../../assets/datasets/terrain/terrain_ceisum_ion_dialog_5.png)

6. 重ね合わされた地形データが完成。必要に応じてプロセスを繰り返す

   ![Cesium ion ダイアログ 6](../../../assets/datasets/terrain/terrain_ceisum_ion_dialog_6.png)

#### 5.3.3. 地形データの配信について

Cesium で地形データを利用するには、地形データをサーバーから配信する必要があります。XYZ タイルデータなどと異なり、データファイルを置いておくだけでは地形データは利用できません。

##### TerriaJS で Cesium ion から配信される地形データを表示する方法

- データカタログに登録する方法

  ```json
  {
    "name": "地形データ",
    "type": "cesium-terrain",
    "ionAssetId": "your_asset_id",
    "ionAccessToken": "your_ion_access_token",
    "description": "地形データは、国土地理院長の承認を得て基盤地図情報数値標高モデルを加工後、配信したものです（承認番号　ＸＸＸＸＸＸＸＸＸ）"
  }
  ```

- デフォルトの地形データとする方法（`config.json` を編集）

  ```json
  {
    "useCesiumIonTerrain": true,
    "cesiumTerrainAssetId": "your_asset_id",
    "cesiumIonAccessToken": "your_ion_access_token"
  }
  ```

## 6. PLATEAU Mapbox Terrain Converter

`terraindb` 形式のデータは Cesium 向けの地形データ形式であり、Mapbox GL JS や MapLibre GL JS などの他の地図エンジンが直接扱えないという課題があります。

そこで、2024 年度の事業において、CityGML 形式の PLATEAU 地形モデル（TIN）を Mapbox や MapLibre で利用可能な地形データである Mapbox Terrain-RGB に変換するライブラリ「PLATEAU Mapbox Terrain Converter」が開発されました。

ライブラリの利用方法および生成した日本全域の地形データの利用方法については、下記のリポジトリを参照してください。

- [PLATEAU Mapbox Terrain Converter](https://github.com/Project-PLATEAU-Admin/plateau-mb-terrain-converter)
