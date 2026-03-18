# PLATEAU アセット命名規則ガイド

このドキュメントは、PLATEAU データカタログで使用されるアセット名の命名規則を説明します。

## 目次

1. [基本構造](#基本構造)
2. [フィールド詳細](#フィールド詳細)
3. [拡張情報（Extension）](#拡張情報extension)
4. [地物タイプ別の例](#地物タイプ別の例)
5. [関連アセット](#関連アセット)

---

## 基本構造

アセット名は以下の形式で構成されます：

```
{CityCode}_{CityName}_{Provider}_{Year}_{Format}_{UpdateCount}[_{Option}]_op[_{Extension}]
```

### 最小構成の例

```
26100_kyoto-shi_city_2022_citygml_3
```

### 拡張情報を含む例

```
26100_kyoto-shi_city_2023_citygml_1_op_bldg_3dtiles_lod2
```

> **補足**: `_op` より前の部分（`{CityCode}_{CityName}_{Provider}_{Year}_{Format}_{UpdateCount}`）は、基本的に変換元CityGMLのzipファイル名がそのまま使用されます。

---

## フィールド詳細

| フィールド | 形式 | 説明 | 例 |
|-----------|------|------|-----|
| CityCode | 5桁の数字 | 市区町村コード（JIS X 0402） | `26100`, `13101` |
| CityName | 小文字英数字・ハイフン | 市区町村名（ローマ字） | `kyoto-shi`, `chiyoda-ku` |
| Provider | 小文字英数字・ハイフン | データ提供者 | `city`, `mlit`, `udx-mlit` |
| Year | 4桁の数字 | 整備年度 | `2022`, `2023` |
| Format | 小文字英字 | ベースフォーマット | `citygml` |
| UpdateCount | 数字 | 更新回数 | `1`, `3` |
| Option | 任意の文字列 | オプション情報（`_op`の前） | `sample-takeshiba` |
| Extension | 拡張情報 | 地物の詳細情報（`_op`の後） | `bldg_3dtiles_lod2` |

### Provider（提供者）の種類

| コード | 説明 |
|--------|------|
| `city` | 市区町村 |
| `mlit` | 国土交通省 |
| `udx-mlit` | 都市局版（国交省） |

---

## 拡張情報（Extension）

拡張情報は `_op` の後に続く部分で、地物の詳細を表します。

CityGMLから3D TilesやMVTなどに変換する際、変換結果は地物タイプやLODごとに細分化されるため、それらを識別するための追加情報として付与されます。

主に2種類の形式があります。

### 通常の地物（AssetNameExNormal）

```
{Type}[_{Name}]_{Format}[_{WardCode}_{WardName}][_lod{N}][_interior][_no_texture]
```

#### フィールド

| フィールド | 必須 | 説明 | 例 |
|-----------|------|------|-----|
| Type | 必須 | 地物タイプコード | `bldg`, `tran`, `urf` |
| Name | 任意 | 地物の名前・サブタイプ | `AreaClassification`, `PlantCover` |
| Format | 必須 | 出力フォーマット | `3dtiles`, `mvt` |
| WardCode | 任意 | 区コード（※） | `26103`, `43101` |
| WardName | 任意 | 区名（※） | `sakyo-ku`, `higashi-ku` |

※ WardCode・WardNameは、建築物モデル（bldg）の3D Tilesでのみ使用されます。政令指定都市の場合、3D Tiles変換時に区ごとに分割されるため付与されます（元のCityGMLでは分割されていません）。

#### 対応フォーマット

| フォーマット | 説明 |
|-------------|------|
| `3dtiles` | 3D Tiles形式 |
| `mvt` | Mapbox Vector Tiles形式 |
| `dm_geometric_attributes` | 測量成果（内部的にmvtとして扱われる） |

#### オプションフラグ

| フラグ | 説明 |
|--------|------|
| `_lod{N}` | LOD（Level of Detail）。整数値 |
| `_lod{N}{M}` | 小数点付きLOD。例: `_lod12` は LOD 1.2 |
| `_interior` | 屋内モデルを含む |
| `_no_texture` | テクスチャなし |

### 浸水想定区域データ（AssetNameExFld）

```
fld_{Admin}_{River}_{Format}_{L}[-{Suffix}][_no_texture]
```

| フィールド | 説明 | 例 |
|-----------|------|-----|
| Admin | 管理区分 | `natl`（国）, `pref`（県） |
| River | 河川名（複数はアンダースコア連結） | `yabegawa_haegawa` |
| Format | 常に `3dtiles` | `3dtiles` |
| L | 浸水ランク | `l1`（計画規模）, `l2`（想定最大規模） |
| Suffix | オプショナルなサフィックス（※） | `p1-0001` |

※ **Suffix について**: 特にtnm（津波浸水想定区域）では、津波浸水想定区域図ごとに変換を行うため、それらを区別するための番号として使用されます。fld（洪水浸水想定区域）でも、稀に一つの河川を複数に分割して変換する場合に使用されることがあります。

※ `_no_texture` は基本的に不要です（浸水想定区域データは元々テクスチャを持たないため）。

---

## 地物タイプ別の例

### bldg（建築物）

**基本形式:**
```
26100_kyoto-shi_city_2023_citygml_1_op_bldg_3dtiles_lod2
```

**区コード付き:**
```
26100_kyoto-shi_city_2023_citygml_1_op_bldg_3dtiles_26103_sakyo-ku_lod2_no_texture
```

**屋内モデル付き:**
```
26100_kyoto-shi_city_2023_citygml_1_op_bldg_3dtiles_lod2_interior
```

**小数点付きLOD:**
```
13999_tokyo_mlit_2023_citygml_1_op_bldg_3dtiles_lod12
```
（LOD 1.2）

### tran（交通）

```
33211_bizen-shi_city_2023_citygml_1_op_tran_mvt_lod1
```

### fld（浸水想定区域）

**基本形式:**
```
40202_omuta-shi_city_2023_citygml_1_op_fld_natl_yabegawa_haegawa_3dtiles_l1
```

**サフィックス付き:**
```
28201_himeji-shi_city_2023_citygml_1_op_fld_natl_ibogawa_hayashidagawa_3dtiles_l2-p1-0001
```

### tnm（津波浸水想定区域）

```
40202_omuta-shi_city_2023_citygml_1_op_tnm_40_1_3dtiles
```

### urf（都市計画決定情報）

```
40202_omuta-shi_city_2023_citygml_1_op_urf_AreaClassification_mvt_lod1
```

### veg（植生）

```
11111_bar-shi_city_2023_citygml_1_op_veg_PlantCover_3dtiles_lod3
```

### frn（都市設備）

**3D Tiles:**
```
15202_nagaoka-shi_city_2023_citygml_1_op_frn_3dtiles_lod2
```

**測量成果:**
```
15202_nagaoka-shi_city_2023_citygml_1_op_frn_dm_geometric_attributes
```
（`dm_geometric_attributes` は内部的に `mvt` LOD 0 として扱われる）

### gen（汎用都市オブジェクト）

```
00000_xxx_city_2023_citygml_1_op_gen_00_mvt_lod0
```

### area（区域）

```
13999_tokyo_mlit_2023_citygml_1_op_area_0202_Zone_mvt_lod1
```

### unf（地下施設）

```
13999_tokyo_mlit_2023_citygml_1_op_unf_Manhole_mvt_lod32
```
（LOD 3.2）

---

## 関連アセット

関連アセットは以下の形式を使用します：

```
{Code}_{Name}_{Provider}_{Year}[_{WardCode}_{WardName}]_{Type}.{Format}
```

### フィールド

| フィールド | 必須 | 説明 | 例 |
|-----------|------|------|-----|
| Code | 必須 | 市区町村コード | `41423`, `13101` |
| Name | 必須 | 市区町村名（ローマ字） | `omachi-cho`, `chiyoda-ku` |
| Provider | 必須 | データ提供者 | `city`, `mlit` |
| Year | 必須 | 整備年度 | `2022`, `2023` |
| WardCode | 任意 | 区コード（政令指定都市の場合） | `43100` |
| WardName | 任意 | 区名（ローマ字） | `higashi-ku` |
| Type | 必須 | 関連データのタイプ | `emergency_route`, `border`, `landmark` |
| Format | 必須 | ファイル拡張子 | `geojson`, `czml` |

### 例

**緊急輸送道路:**
```
41423_omachi-cho_city_2022_emergency_route.geojson
```

**境界線:**
```
13101_chiyoda-ku_city_2023_border.czml
```

**区コード付きランドマーク:**
```
43100_kumamoto-shi_city_2022_43100_higashi-ku_landmark.czml
```

### 対応フォーマット

| 拡張子 | 説明 |
|--------|------|
| `.geojson` | GeoJSON形式 |
| `.czml` | CZML形式（Cesium用） |

---

## オプション付きアセット

オプションは `_op` の前に配置されます：

```
13999_tokyo_udx-mlit_2023_citygml_1_sample-takeshiba_op
```

オプションと拡張情報の両方を含む例：

```
13999_tokyo_udx-mlit_2023_citygml_1_sample-takeshiba_op_bldg_3dtiles_lod2
```

---

## 注意事項

1. **大文字・小文字**: 基本フィールドは小文字を使用。ただし、河川名や地物名（Name）は大文字を含む場合があります。

2. **ハイフンとアンダースコア**:
   - 単語の区切りにはハイフン（`-`）を使用（例: `kyoto-shi`）
   - フィールドの区切りにはアンダースコア（`_`）を使用

3. **LODの表記**:
   - 整数LOD: `_lod1`, `_lod2`, `_lod3`
   - 小数点付きLOD: `_lod12` = LOD 1.2

4. **浸水想定区域データの特殊性**: 浸水想定区域データ（fld）は他の地物と異なる形式を使用します。特に浸水ランク（l1, l2）の表記に注意してください。

## 参考

- [3D都市モデル標準製品仕様書 7.2.3	ファイル名称](https://www.mlit.go.jp/plateaudocument/toc7/toc7_02/toc7_02_03/)
