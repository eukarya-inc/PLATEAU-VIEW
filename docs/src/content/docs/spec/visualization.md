---
title: PLATEAU 2023 可視化用データ変換仕様
description: 3D 都市モデル標準製品仕様書 3.x 版に基づく CityGML から 3D Tiles / MVT への変換仕様
---

## 本仕様書の目的

本仕様書は、3D 都市モデル標準製品仕様書 3.x 版に基づいて整備された CityGML 形式の 3D 都市モデルデータを Cesium 環境で可視化するための Cesium 3D Tiles（以下「3D Tiles」）または Mapbox Vector Tile（以下「MVT」）データセットへの変換仕様（主に主題属性の構成）を説明するものです。

1. 変換対象の地物型と変換結果データセット名
2. 3D Tiles と MVT の主題属性の構成
3. 変換に使用するソフトウェア等

## 1. 変換対象の地物型と変換結果データセット名

標準製品仕様書 3.x 版で定義されている空間属性を持つ地物型のうち、地形モデルを除くすべての地物型を可視化のためのデータ変換の対象とします。CityGML および i-UR の仕様に基づくフィーチャークラスについては詳細度別に、実質的に 2D の空間属性を持つもの（標準製品仕様書の規定に基づいて z=0 で作成されるもの）は MVT に、3D の空間属性を持つものは 3D Tiles に変換します。

公共測量標準図式の応用スキーマに基づく `uro:DmGeometricAttribute` 型（空間属性を持つもの）および `uro:DmAnnotation` 型の属性として記述されたフィーチャー（以下「DM フィーチャー」）は、すべて MVT に変換します。

### 表 1 可視化用変換結果データセット名

| # | パッケージ | 変換結果データセット名 | データ形式 | 概要 |
|---|---|---|---|---|
| 1 | 建築物モデル | `bldg_lod1` | 3D Tiles | LOD1 建築物モデル |
| | | `bldg_lod2` | 3D Tiles | LOD2 以下の最大詳細度の建築物モデル |
| | | `bldg_lod3` | 3D Tiles | LOD3 以下の最大詳細度の建築物モデル |
| | | `bldg_lod4` | 3D Tiles | LOD4 以下の最大詳細度の建築物モデル |
| | | `bldg_lod1_no_texture` | 3D Tiles | LOD1 建築物モデル（テクスチャなし） |
| | | `bldg_lod2_no_texture` | 3D Tiles | LOD2 以下の最大詳細度の建築物モデル（テクスチャなし） |
| | | `bldg_lod3_no_texture` | 3D Tiles | LOD3 以下の最大詳細度の建築物モデル（テクスチャなし） |
| | | `bldg_lod4_no_texture` | 3D Tiles | LOD4 以下の最大詳細度の建築物モデル（テクスチャなし） |
| | | `bldg_dm_geometric_attributes` | MVT | DmGeometricAttribute に記述されているジオメトリ |
| | | `bldg_dm_annotations` | MVT | DmAnnotation に記述されているジオメトリ |
| 2 | 交通（道路）モデル | `tran_lod0` | MVT | LOD0 道路モデル |
| | | `tran_lod1` | MVT | LOD1 道路モデル |
| | | `tran_lod2` | MVT | LOD2 道路モデル（交通領域、交通補助領域） |
| | | `tran_lod3` | 3D Tiles | LOD3 道路モデル（交通領域、交通補助領域） |
| | | `tran_dm_geometric_attributes` | MVT | DmGeometricAttribute |
| | | `tran_dm_annotations` | MVT | DmAnnotation |
| 3 | 交通（鉄道）モデル | `rwy_lod0`〜`rwy_lod3` | MVT / 3D Tiles | LOD0〜3 鉄道モデル（LOD3 のみ 3D Tiles） |
| | | `rwy_dm_geometric_attributes` / `rwy_dm_annotations` | MVT | DM フィーチャー |
| 4 | 交通（徒歩道）モデル | `trk_lod0`〜`trk_lod3` | MVT / 3D Tiles | LOD0〜3 徒歩道モデル（LOD3 のみ 3D Tiles） |
| | | `trk_dm_geometric_attributes` / `trk_dm_annotations` | MVT | DM フィーチャー |
| 5 | 交通（広場）モデル | `squr_lod0`〜`squr_lod3` | MVT / 3D Tiles | LOD0〜3 広場モデル（LOD3 のみ 3D Tiles） |
| | | `squr_dm_geometric_attributes` / `squr_dm_annotations` | MVT | DM フィーチャー |
| 6 | 交通（航路）モデル | `wwy_lod0`〜`wwy_lod2` | MVT | LOD0〜2 航路モデル |
| 7 | 土地利用モデル | `luse` | MVT | LOD1 土地利用モデル |
| 8 | 災害リスク（浸水）モデル - 洪水 | `fld_{admin}_{river}_{scale}` | 3D Tiles | LOD1 洪水浸水想定区域モデル（補足説明 1） |
| 9 | 災害リスク（浸水）モデル - 津波 | `tnm_{name}` | 3D Tiles | LOD1 津波浸水想定区域モデル（補足説明 2） |
| 10 | 災害リスク（浸水）モデル - 高潮 | `htd_{name}` | 3D Tiles | LOD1 高潮浸水想定区域モデル（補足説明 2） |
| 11 | 災害リスク（浸水）モデル - 内水 | `ifld_{name}` | 3D Tiles | LOD1 内水浸水想定区域モデル（補足説明 2） |
| 12 | 災害リスク（土砂災害）モデル | `lsld` | MVT | LOD1 土砂災害警戒区域モデル |
| 13 | 都市計画決定情報モデル | `urf_{class}` | MVT | LOD1 都市計画決定情報モデル（補足説明 3） |
| 14 | 橋梁モデル | `brid_lod1`〜`brid_lod4` | 3D Tiles | LOD1〜4 橋梁モデル |
| | | `brid_dm_geometric_attributes` / `brid_dm_annotations` | MVT | DM フィーチャー |
| 15 | トンネルモデル | `tun_lod1`〜`tun_lod4` | 3D Tiles | LOD1〜4 トンネルモデル |
| | | `tun_dm_geometric_attributes` / `tun_dm_annotations` | MVT | DM フィーチャー |
| 16 | その他の構造物モデル | `cons_lod0` | MVT | LOD0 その他の構造物モデル |
| | | `cons_lod1`〜`cons_lod3` | 3D Tiles | LOD1〜3 その他の構造物モデル |
| | | `cons_dm_geometric_attributes` / `cons_dm_annotations` | MVT | DM フィーチャー |
| 17 | 都市設備モデル | `frn_lod1`〜`frn_lod3` | 3D Tiles | LOD1〜3 都市設備モデル |
| | | `frn_dm_geometric_attributes` / `frn_dm_annotations` | MVT | DM フィーチャー |
| 18 | 地下埋設物モデル | `unf_{class}_lod1`〜`unf_{class}_lod4` | 3D Tiles | LOD1〜4 地下埋設物モデル（補足説明 4） |
| | | `unf_{class}_dm_geometric_attributes` / `unf_{class}_dm_annotations` | MVT | DM フィーチャー |
| 19 | 地下街モデル | `ubld_lod0` | MVT | LOD0 地下街モデル |
| | | `ubld_lod1`〜`ubld_lod4` | 3D Tiles | LOD1〜4 地下街モデル |
| 20 | 植生モデル | `veg_{class}_lod1`〜`veg_{class}_lod3` | 3D Tiles | LOD1〜3 植生モデル（補足説明 5） |
| | | `veg_{class}_dm_geometric_attributes` / `veg_{class}_dm_annotations` | MVT | DM フィーチャー |
| 21 | 地形モデル | （PLATEAU VIEW での可視化対象外） | | |
| 22 | 水部モデル | `wtr_lod0` | MVT | LOD0 水部モデル |
| | | `wtr_lod1`〜`wtr_lod3` | 3D Tiles | LOD1〜3 水部モデル |
| | | `wtr_dm_geometric_attributes` / `wtr_dm_annotations` | MVT | DM フィーチャー |
| 23 | 区域モデル | `area_Zone` | MVT | LOD1 区域モデル |
| 24 | 汎用都市オブジェクトモデル | `gen_{code}_lod0` | MVT | LOD0 汎用都市オブジェクトモデル（補足説明 6） |
| | | `gen_{code}_lod1`〜`gen_{code}_lod4` | 3D Tiles | LOD1〜4 汎用都市オブジェクトモデル（補足説明 6） |

#### 補足説明 1: 災害リスクモデル（浸水想定区域）洪水

- `fld_{admin}_{river}_{scale}`
  - `{admin}`: 河川管理者区分に応じて `natl`（国）または `pref`（都道府県）
  - `{river}`: 入力データセットにおけるサブフォルダー名（水系名・河川名）
  - `{scale}`: 洪水規模に応じて `l1`（計画規模）または `l2`（想定最大規模）

#### 補足説明 2: 災害リスクモデル（浸水想定区域）津波、高潮、内水

- `(tnm|htd|ifld)_{name}`
  - `{name}`: 入力データセットにおけるサブフォルダー名（浸水想定区域の名称）

#### 補足説明 3: 都市計画決定情報モデル

- `urf_{class}`
  - `{class}`: 当該データセットに格納されるフィーチャークラス名（`UrbanPlanningArea`, `UseDistrict`, `FirePreventionDistrict` など）

#### 補足説明 4: 地下埋設物モデル

- `unf_{class}_(lod[1-4]|dm_geometric_attributes|dm_annotations)`
  - `{class}`: 当該データセットに格納されるフィーチャークラス名（`WaterPipe`, `Duct`, `Manhole` など）

#### 補足説明 5: 植生モデル

- `veg_{class}_(lod[1-3]|dm_geometric_attributes|dm_annotations)`
  - `{class}`: フィーチャークラス名（`SolitaryVegetationObject`（単独木）または `PlantCover`（植被））

#### 補足説明 6: 汎用都市オブジェクトモデル

- `gen_{code}_lod[0-4]`
  - `{code}`: 当該データセットに格納される汎用都市オブジェクトモデルフィーチャーの `gml:name` の値

## 2. 3D Tiles と MVT の主題属性の構成

### 2.1. 共通属性

下表に掲げる属性は原則としてすべての 3D Tiles フィーチャー、MVT フィーチャーに付加します。ただし、災害リスク（浸水）モデルについては、可視化にあたって軽量化を行う都合上、`meshcode`, `gml_id` 属性は付加しません。

#### 表 2 共通属性

| # | 属性名 | 内容 |
|---|---|---|
| 1 | `meshcode` | メッシュコード。入力 GML ファイル名を `_` 区切りで分割した場合の最初の要素 |
| 2 | `city_code` | 市区町村コード（5 桁数字） |
| 3 | `city_name` | 市区町村名（標準のコードリスト `Common_localPublicAuthorities.xml` に基づく） |
| 4 | `feature_type` | CityGML i-UR で定義されているフィーチャータイプ（地物型）名 |
| 5 | `gml_id` | 各フィーチャーの `gml:id` 属性 |
| 6 | `attributes` | 全ての主題属性を元の XML の階層構造に準じて格納した JSON 文書 |

### 2.2. 地物型別のフラットにする属性

上記共通属性のほか、標準製品仕様書 3.2 版付属 `template_objectlist.xlsx` ` [A.3.1_取得項目一覧]` シートにおいて「拡張製品仕様書の対象とすべき主題属性」として「●：データ作成上必須」または「○：原則として入力」と指定されている主題属性を抽出し、該当するフィーチャーにそれぞれ単独の属性として付加します。

詳細は「PLATEAU2023 データ変換仕様【別添】地物型別のフラットにする属性.xlsx」を参照してください。

### 2.3. DM フィーチャーの属性

DM フィーチャーには、上記の共通属性および地物型別の固有の属性に加え、さらに表 3、表 4 に掲げる属性を追加します。

#### 表 3 `uro:DmGeometricAttribute` フィーチャーに追加する属性

| # | 属性名 | 内容 |
|---|---|---|
| 1 | `dm_attributes` | 全属性を XML の階層構造に準じた構造で格納した JSON 文書 |
| 2 | `dm_dmCode` | DM コードの意味文字列 |
| 3 | `dm_dmCode_code` | DM コード |
| 4 | `dm_geometryType` | レコードタイプ |
| 5 | `dm_geometryType_code` | レコードタイプコード |
| 6 | `dm_mapLevel` | 地図情報レベル |
| 7 | `dm_mapLevel_code` | 地図情報レベルコード |
| 8 | `dm_shapeType` | 図形区分 |
| 9 | `dm_shapeType_code` | 図形区分コード |

#### 表 4 `uro:DmAnnotation` フィーチャーに追加する属性

| # | 属性名 | 内容 |
|---|---|---|
| 1 | `dm_attributes` | 全属性を XML の階層構造に準じた構造で格納した JSON 文書 |
| 2 | `dm_dmCode` | DM コードの意味文字列 |
| 3 | `dm_dmCode_code` | DM コード |
| 4 | `dm_geometryType` | レコードタイプ |
| 5 | `dm_geometryType_code` | レコードタイプコード |
| 6 | `dm_shapeType` | 図形区分 |
| 7 | `dm_shapeType_code` | 図形区分コード |
| 8 | `dm_label` | 注記文字列 |
| 9 | `dm_isVertical` | 文字方向 |
| 10 | `dm_size` | 字大 |
| 11 | `dm_orientation` | 角度 |
| 12 | `dm_linewidth` | 線号 |
| 13 | `dm_spacing` | 字隔 |

## 3. 変換に使用するソフトウェア等

可視化用のデータ変換は、FME（カナダ Safe Software Inc. 製 2022.2.6 以降のバージョン）によって表に掲げるワークスペースを実行することにより行います。

### 表 5 可視化用データ変換ワークスペース一覧

| # | ワークスペース名 | 備考 |
|---|---|---|
| 1-1 | PLATEAU3 可視化用データ変換 01 建築物.fmw | |
| 1-2 | PLATEAU3 可視化用データ変換 01-2 政令市の建築物.fmw | 注 1 |
| 2 | PLATEAU3 可視化用データ変換 02 道路・鉄道・徒歩道・広場・航路.fmw | |
| 3 | PLATEAU3 可視化用データ変換 03 都市設備・植生.fmw | |
| 4 | PLATEAU3 可視化用データ変換 04 土地利用・土砂災害警戒区域.fmw | |
| 5 | PLATEAU3 可視化用データ変換 05 浸水想定区域.fmw | |
| 6 | PLATEAU3 可視化用データ変換 06 都市計画決定情報・区域.fmw | |
| 7 | PLATEAU3 可視化用データ変換 07 橋梁・トンネル・その他の構造物.fmw | |
| 8 | PLATEAU3 可視化用データ変換 08 地下街.fmw | |
| 9 | PLATEAU3 可視化用データ変換 09 地下埋設物.fmw | |
| 10 | PLATEAU3 可視化用データ変換 10 水部.fmw | |
| 11 | PLATEAU3 可視化用データ変換 11 汎用都市オブジェクト.fmw | |

注 1: 政令市の建築物モデルの変換にあたっては、#1-2 ワークスペースを実行することにより区ごとに #1-1 を実行して区単位の 3D Tiles を作成することができます。

これらのワークスペース（テンプレート）は FME Hub（<https://hub.safe.com>）で公開しており、ウェブブラウザによって同サイトからダウンロードできるほか、FME Workbench メニューの **File → Workspace from Template** によって直接ダウンロードすることも可能です。

なお、3D Tiles データセットへの変換を行うワークスペースを実行するには「日本のジオイド 2011」ジオイドモデルに基づく楕円体高への変換を行うため、FME Hub で公開されている [Vertical Transformation with GSIGEO2011](https://hub.safe.com/publishers/pacific-spatial-solutions/templates/vertical-transformation-with-gsigeo2011) テンプレートに同梱されているグリッドファイルが必要です。
