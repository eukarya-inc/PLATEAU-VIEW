---
title: PLATEAU-Terrain
description: 日本全国の地形データ配信サービスの利用方法
---

## 1. PLATEAU-Terrain の概要

Project PLATEAU では、日本全国の地形データを Cesium などの 3D 地図エンジンから直接利用できるタイル配信サービス「PLATEAU-Terrain」を提供しています。

国土地理院の基盤地図情報数値標高モデル（DEM）をはじめとした各種データソースから生成された標高値に、日本のジオイドモデル（[GSIGEO2011](https://www.gsi.go.jp/buturisokuchi/grageo_geoidseika.html) など）を合成し、**楕円体高（ellipsoidal height）** に変換した地形を配信しています。これにより、3D Tiles などのジオコード済みデータと垂直方向のずれが発生しないようになっています。

本チュートリアルでは、PLATEAU-Terrain の利用方法について解説します。

:::note[現在の提供状況]
現在配信しているのは、Cesium 向けの **quantized-mesh 形式**の地形タイル（`/terrain/`）です。これは PLATEAU-Terrain の地形タイルをオブジェクトストレージから配信しているものです。配信される標高は楕円体高に変換済みですが、**ジオイドモデルはデータ生成時に固定**されており、リクエスト時に切り替えることはできません。

リクエスト時の高さ基準の切り替え（`?heights=`）や、MapLibre / Mapbox GL 向けの raster-dem エンドポイント（`/terrarium/`・`/mapbox/`）は、新しい DEM ベースの配信パイプラインで開発を進めていますが、**現在調整中で、まだリリースされていません**（[1.3](#13-今後提供予定の機能調整中)）。
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

- **高さの基準の切り替え**：`?heights=` クエリパラメータで、配信する鉛直方向の面を切り替え。

  | `heights=` | 配信される高さ |
  | --- | --- |
  | `ellipsoidal`（既定） | 楕円体高（正標高 + ジオイド高） |
  | `orthometric` | 正標高（ジオイド高を加算しない、DEM そのまま） |
  | `geoid` | ジオイド高そのもの |

  ジオイドモデル（[GSIGEO2011](https://www.gsi.go.jp/buturisokuchi/grageo_geoidseika.html) や日本のジオイド 2024 など）は、**データセットごとに固定**でリクエストからは選べません。ジオイドモデルはそれぞれ特定の測地成果と対になっており、DEM の測地系に合わないモデルを重ねても意味のある高さにならないためです。適用されているモデルはデータセットの仕様として公開します。

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
現在の `/terrain/` は、固定のジオイドモデルを適用済みの quantized-mesh ミラーです。`?heights=` を付与しても高さの基準は切り替わりません（[1.3](#13-今後提供予定の機能調整中)）。
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

アクセストークンは不要です。`CesiumTerrainProvider.fromUrl` に `/terrain/` の URL を渡すだけで動作します。

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

PLATEAU-Terrain を利用する場合は、地図画面上に下記の帰属を必ず表示してください。

```
PLATEAU | 国土地理院
```

`/terrain/layer.json` のレスポンスにも同等の `attribution` 文字列が含まれます。

あわせて、データの帰属に「地形データは、測量法に基づく国土地理院長承認（使用）R3JHs 778 を得て使用」と記載してください。

## 5. PLATEAU Mapbox Terrain Converter

quantized-mesh 形式のデータは Cesium 向けの地形データ形式であり、Mapbox GL JS や MapLibre GL JS などの他の地図エンジンが直接扱えないという課題があります。

そこで、2024 年度の事業において、CityGML 形式の PLATEAU 地形モデル（TIN）を Mapbox や MapLibre で利用可能な地形データである Mapbox Terrain-RGB に変換するライブラリ「PLATEAU Mapbox Terrain Converter」が開発されました。

ライブラリの利用方法および生成した日本全域の地形データの利用方法については、下記のリポジトリを参照してください。

- [PLATEAU Mapbox Terrain Converter](https://github.com/Project-PLATEAU-Admin/plateau-mb-terrain-converter)
