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

レスポンスの `url` フィールドが配信 URL です。例えば 3D Tiles なら `tileset.json` の URL、MVT なら `{z}/{x}/{y}.mvt` 形式の URL です。

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
      'https://assets.cms.plateau.reearth.io/assets/0e/e5948a-e95c-4e31-be85-1f8c066ed996/13101_chiyoda-ku_pref_2023_citygml_1_op_bldg_3dtiles_13101_chiyoda-ku_lod1/tileset.json'
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
