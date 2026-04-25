---
title: クイックスタート
description: PLATEAU 配信サービスから最初のデータを取得する最短手順
---

このページでは、PLATEAU 配信サービスから最初のデータを取得するまでの最短手順を紹介します。

## 1. データを探す

API に慣れていない方は、まず [**データセット一覧**](/datasets/explorer/) ページがおすすめです。
都道府県・市区町村・種別・形式などで絞り込み、配信 URL をその場でコピーできます。
コピーした URL は、次の「3. 表示する」のサンプルコードにそのまま貼り付けて使えます。

API から直接取得したい場合は、データカタログ API を使ってデータを検索します。例えば、東京都千代田区（自治体コード `13101`）のデータセット一覧を取得：

```bash
curl 'https://api.plateauview.mlit.go.jp/datacatalog/plateau-datasets' \
  | jq '.datasets[] | select(.city_code == "13101")'
```

または GraphQL でも取得できます：

```bash
curl -X POST https://api.plateauview.mlit.go.jp/datacatalog/graphql \
  -H 'Content-Type: application/json' \
  -d '{"query":"{ area(code: \"13101\") { datasets { name items { url format } } } }"}'
```

## 2. 配信 URL を取得する

レスポンスの `composite_url` フィールドが、CesiumJS や MapLibre などのクライアントから利用しやすい配信 URL です。
- 3D Tiles の場合: 自治体単位の `tileset.json` を動的生成した URL（[複合 tileset.json API](/datasets/3d-tiles/#42-複合-tilesetjson複数都市の-3d-tiles-を-1-つの-url-でまとめて取得)）
- MVT の場合: 自治体単位の TileJSON 3.0 を動的生成した URL（[MVT TileJSON API](/datasets/3d-tiles/#43-自治体単位の-mvt-tilejsonmaplibre-などから利用)）

`url` フィールドは原典となる配信 URL（3D Tiles なら `tileset.json`、MVT なら `{z}/{x}/{y}.mvt` 形式）で、PLATEAU CMS から直接配信されます。

:::tip[`composite_url` を強く推奨します]
PLATEAU の都市データは毎年更新され、新しい整備年度のデータが公開されます。`url`（CMS 直リンク）を埋め込むと、新しい年度が公開されたときにアプリケーション側のコード書き換えが必要になります。

`composite_url` は API サーバ側でデータを動的に解決するため、**特に整備年度を `latest` で指定した形**（例: `13101-bldg-lod2-latest`）を使えば、新しいデータが公開されたタイミングで URL を変更しなくても自動的に最新データに追従します。広域用の `composite_tilesets` 配列の `-latest` エントリも同様です。
:::

## 3. 表示する

最も手軽なのは CesiumJS での 3D Tiles 表示です。

```html
<!doctype html>
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <script src="https://cesium.com/downloads/cesiumjs/releases/1.117/Build/Cesium/Cesium.js"></script>
  <link href="https://cesium.com/downloads/cesiumjs/releases/1.117/Build/Cesium/Widgets/widgets.css" rel="stylesheet">
  <style>html, body, #cesiumContainer { width:100%; height:100%; margin:0; padding:0; overflow:hidden; }</style>
</head>
<body>
  <div id="cesiumContainer"></div>
  <script>
    const viewer = new Cesium.Viewer("cesiumContainer", {});
    Cesium.Cesium3DTileset.fromUrl(
      'https://api.plateauview.mlit.go.jp/datacatalog/3dtiles/13101-bldg-lod1-latest/tileset.json'
    ).then(tileset => {
      viewer.scene.primitives.add(tileset);
      viewer.zoomTo(tileset);
    });
  </script>
</body>
</html>
```

このファイルをブラウザで開けば、千代田区の 3D 建物モデルが表示されます。

## 次のステップ

- データセットの種類別の詳しい使い方は [データセット](/datasets/citygml/) セクションを参照
- AI クライアントから利用するなら [PLATEAU MCP Server](/mcp/overview/)
- 全データセット仕様は [可視化用データ変換仕様](/spec/visualization/)
