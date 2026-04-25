---
title: PLATEAU-Terrain
description: 地形モデルの terraindb / Terrain-RGB 配信仕様と利用方法
---

## 1. PLATEAU-Terrain の概要

Project PLATEAU では、Cesium をベースに作られた PLATEAU VIEW で利用する日本全国の地形データ「PLATEAU-Terrain」を作成し、配信を行っています。

また PLATEAU が提供する CityGML 形式の地形データを、Mapbox Terrain-RGB に変換する「PLATEAU Mapbox Terrain Converter」を開発し、それを用いて作成した日本全国の地形データの配信も行っています。

本チュートリアルでは、地形データ作成技術および利用方法について解説します。

### 1.1. 地形データについて

地形データとは、地形を 3 次元でモデル化したデータです。通常、地形データはほかの 3 次元データを重畳するための基盤データとして利用され、地形データ自体を視覚的に表現することはありません。

PLATEAU VIEW では、地形データの上に、地理院タイルやオルソ写真、そのほか 2 次元のタイルデータをドレープ（覆いかぶせる処理）して地表面を表示しています。地形データとは、3 次元データを表示する際の骨格のようなデータということができます。

![地形データのイメージ](../../../assets/datasets/terrain/terrain_image.png)

図 1 地形データのイメージ（引用: <https://cesium.com/blog/2015/12/18/terrain-quantization/>）

地形データの種類やデータフォーマットには様々なものがあります。これらのうち、PLATEAU VIEW を構成する Cesium では、[quantized-mesh 形式](https://github.com/CesiumGS/quantized-mesh)のデータを利用しています。

Quantized-mesh 形式は、3D のメッシュデータを量子化（quantization）して圧縮する方式で、通常のメッシュデータよりもデータサイズと使用メモリ量を大幅に縮小することができます。

#### 地形データの種類

- 標高タイル: <https://maps.gsi.go.jp/development/demtile.html>
- Mapbox Terrain-RGB v1: <https://docs.mapbox.com/data/tilesets/reference/mapbox-terrain-dem-v1/>
- Quadtree（四分木）: <https://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.69.7733&rep=rep1&type=pdf>
- QuadTin: <https://www.ifi.uzh.ch/dam/jcr:ffffffff-d894-8e94-0000-0000604fd2b6/QuadTIN.pdf>
- Quantized-mesh: <https://github.com/CesiumGS/quantized-mesh>
- TIN（triangulated irregular network）: <https://ja.wikipedia.org/wiki/TIN>

### 1.2. PLATEAU-Terrain の概要

PLATEAU VIEW では、quantized-mesh 形式のデータを `.terraindb` 形式のファイルとして作成した PLATEAU-Terrain を利用しています。

PLATEAU-Terrain は、国土地理院が整備した基盤地図情報数値標高モデル 5m メッシュを基本とし、5m メッシュが存在しない場所は基盤地図情報数値標高モデル 10m メッシュを利用して作成されています。また、ジオイドモデルには「日本のジオイド 2011 (Ver.2.2)」を使用しています。

詳細なデータ作成方法については「3. 地形データの作成」を参照してください。

## 2. PLATEAU-Terrain の利用方法

### 2.1. アクセストークンおよびアセット ID

PLATEAU-Terrain を Cesium で利用する際は以下のトークンとアセット ID を利用してください。

:::caution
本サービスはあくまで試験的な運用であるため、提供期間やサービスレベルについては保証できないことをご了承ください。
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

### 2.2. CesiumJS アプリケーションの作成

CesiumJS 上で PLATEAU-Terrain を利用するためのサンプルコードを示します。

地形データの配信についてご質問がある方は、PacificSpatialSolutions 株式会社（info@pacificspatial.com）までご連絡ください。

配信された地形データを利用する場合は、「地形データは、測量法に基づく国土地理院長承認（使用）R3JHs 778 を得て使用」とデータの帰属に記載してください。

なお、建物モデルは [PLATEAU-3DTiles](/datasets/3d-tiles/) から配信されている千代田区の建物モデルデータを、ドレープするオルソ写真は [PLATEAU-Ortho](/datasets/ortho/) から配信されている東京都 23 区の航空写真を参照しています。

```html
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <title>PLATEAU-3DTiles/MVT、PLATEAU-Ortho、PLATEAU-Terrain を Cesium で表示</title>
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

:::note
CesiumJS では、PLATEAU-Terrain 以外に、地形データとして Cesium がデフォルトで配信している Cesium World Terrain を利用することもできます。日本地域における地形データ詳細度は、Cesium World Terrain のほうが PLATEAU-Terrain より劣りますが、世界各地で地形表現を有効にできます。
:::

```js
const viewer = new Cesium.Viewer('cesiumContainer', {
  terrainProvider: Cesium.createWorldTerrain()
});
```

## 3. 地形データの作成

PLATEAU-Terrain などのオープンデータとして利用できる地形データを利用することに加え、独自に地形データを作成することも可能です。

本節では、独自に地形データを整備するために必要なデジタル標高モデル（DEM）データの作成方法について説明します。

### 3.1. DEM データの作成

地形データを作成する場合は、地形データのもととなるラスター形式のデジタル標高モデル（DEM）を準備する必要があります。

ここでは、国土地理院が整備する基盤地図情報数値標高モデルを利用し、FME を用いて DEM データを作成する方法について説明します。

なお、DEM データは様々な方法で作成可能であり、必ずしも FME を利用する必要はありません。

#### 3.1.1. 作業フォルダと FME ワークスペースの格納

基盤地図情報数値標高モデルから地形モデル用 GeoTIFF 形式 DEM ラスターへの変換を行うため、以下のワークスペースをダウンロードして作業フォルダ内に格納してください。

- [s1_基盤地図情報DEMのGeoTIFF変換.fmw](/fme/s1_基盤地図情報DEMのGeoTIFF変換.fmw)
- [s1_基盤地図情報DEMのGeoTIFF変換_runner.fmw](/fme/s1_基盤地図情報DEMのGeoTIFF変換_runner.fmw)
- [s2_海面つきDEM10B_GeoTIFF作成.fmw](/fme/s2_海面つきDEM10B_GeoTIFF作成.fmw)

これらの FME ワークスペースは、以下の条件で稼働することを確認しています。

- FME 2021.2.2 またはそれ以降のバージョンの FME Desktop
- FME Hub で公開されているカスタムフォーマット [Japanese Fundamental Geospatial Data (FGD) DEM V2](https://hub.safe.com/publishers/pacific-spatial-solutions/formats/japanese-fundamental-geospatial-data-fgd-dem-v2) のダウンロードとインストール
- ダウンロードした `JP_FGD_DEM2.fds` をエクスプローラー上で右クリックしてインストール

#### 3.1.2. DEM データのダウンロード

基盤地図情報ダウンロードサイトから対象とする地域の数値標高モデル（DEM）データ（5A, 5B, 5C, 10A, 10B）をすべてダウンロードし、作業フォルダーに保存してください。

基盤地図情報 DEM の各データセット（2 次メッシュ区画単位）の zip ファイルは解凍しないでください。基盤地図情報ダウンロードサイトの「まとめてダウンロード」機能を使って複数のデータセット（zip）をさらにまとめた zip ファイルでダウンロードした場合には、最上位の zip のみ解凍して 2 次メッシュ区画単位のデータセットに展開しておいてください。

DEM の種類（5A, 5B, 5C, 10A, 10B）ごとにサブフォルダーに分ける必要はありません。

#### 3.1.3. FME ワークスペースの実行

##### (1) 基盤地図情報 DEM データから GeoTIFF 形式 DEM ラスターへの変換

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

##### (2) 海域に標高値を与えた DEM ラスター（10B）の作成

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

### 3.2. 地形データへの変換

ここでは、Cesium ion を利用した DEM データの地形データへの変換方法を説明します。

Cesium ion のサービスを利用することで、データの変換後、地形データの配信も行えるようになります。なお、Cesium ion は有償サービスですが、一定の範囲内であれば無償で利用可能です。

Cesium ion では、ユーザーがアップロードした DEM ファイルを `terraindb` 形式に変換し、配信に利用可能です。`terraindb` 形式への変換は、Cesium ion のクラウドサービスに加え、Cesium のオンプレミス（有償）で提供される変換プログラムでも行えます。日本全国の詳細な地形データを作成する場合は、Cesium のオンプレミスの利用をお勧めします。

#### 3.2.1. Cesium ion アカウント開設

まず、[Cesium ion](https://cesium.com/) のアカウントを用意します。データサイズが 50 GB を超える場合、有料の Commercial アカウント以上を利用する必要があります。

#### 3.2.2. 作成した DEM のアップロードと変換

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

### 3.3. 地形データの配信について

Cesium で地形データを利用するには、地形データをサーバーから配信する必要があります。XYZ タイルデータなどと異なり、データファイルを置いておくだけでは地形データは利用できません。

#### TerriaJS で Cesium ion から配信される地形データを表示する方法

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

## 4. PLATEAU Mapbox Terrain Converter

これまで説明した `terraindb` 形式のデータは Cesium 向けの地形データ形式ですが、Mapbox GL JS や MapLibre GL JS などの他の地図エンジンは対応していないという課題があります。

そこで、2024 年度の事業において、CityGML 形式の PLATEAU 地形モデル（TIN）を Mapbox や MapLibre で利用可能な地形データである Mapbox Terrain-RGB に変換するライブラリ「PLATEAU Mapbox Terrain Converter」を開発しました。

ライブラリの利用方法および生成した日本全域の地形データの利用方法については、下記のリポジトリを参照してください。

- [PLATEAU Mapbox Terrain Converter](https://github.com/Project-PLATEAU-Admin/plateau-mb-terrain-converter)
