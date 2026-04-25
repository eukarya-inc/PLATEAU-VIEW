---
title: GraphQL スキーマリファレンス
description: PLATEAU データカタログ API の GraphQL スキーマ全型定義
tableOfContents:
  maxHeadingLevel: 4
---

:::note[自動生成]
このページは PLATEAU データカタログ GraphQL API の introspection から自動生成されています。実装の真実の源は [`/datacatalog/graphql` エンドポイント](https://api.plateauview.mlit.go.jp/datacatalog/graphql) です。対話的にクエリを試したい場合は [プレイグラウンド](/api/graphql/playground/) を使ってください。
:::


### Query
PLATEAU GraphQL API のクエリルート。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="query.node">node</strong></td>
<td valign="top"><a href="#node">Node</a></td>
<td>

指定されたIDでオブジェクトを取得します。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">id</td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="query.nodes">nodes</strong></td>
<td valign="top">[<a href="#node">Node</a>]!</td>
<td>

指定されたIDのリストからオブジェクトを検索します。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">ids</td>
<td valign="top">[<a href="#id">ID</a>!]!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="query.area">area</strong></td>
<td valign="top"><a href="#area">Area</a></td>
<td>

地域コード（都道府県コードや市区町村コード）で地域を取得します。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">code</td>
<td valign="top"><a href="#areacode">AreaCode</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="query.areas">areas</strong></td>
<td valign="top">[<a href="#area">Area</a>!]!</td>
<td>

地域を検索します。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">input</td>
<td valign="top"><a href="#areasinput">AreasInput</a></td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="query.datasettypes">datasetTypes</strong></td>
<td valign="top">[<a href="#datasettype">DatasetType</a>!]!</td>
<td>

データセットの種類を検索します。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">input</td>
<td valign="top"><a href="#datasettypesinput">DatasetTypesInput</a></td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="query.datasets">datasets</strong></td>
<td valign="top">[<a href="#dataset">Dataset</a>!]!</td>
<td>

データセットを検索します。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">input</td>
<td valign="top"><a href="#datasetsinput">DatasetsInput</a></td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="query.plateauspecs">plateauSpecs</strong></td>
<td valign="top">[<a href="#plateauspec">PlateauSpec</a>!]!</td>
<td>

利用可能な全てのPLATEAU都市モデルの仕様を取得します。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="query.years">years</strong></td>
<td valign="top">[<a href="#int">Int</a>!]!</td>
<td>

利用可能な全てのデータセットの年度（西暦）を取得します。

</td>
</tr>
</tbody>
</table>

### Objects

#### City

市区町村

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="city.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.type">type</strong></td>
<td valign="top"><a href="#areatype">AreaType</a>!</td>
<td>

地域の種類

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.code">code</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a>!</td>
<td>

市区町村コード。先頭に都道府県コードを含む5桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

市区町村名

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.prefectureid">prefectureId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

市区町村が属する都道府県のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.prefecturecode">prefectureCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a>!</td>
<td>

市区町村が属する都道府県コード。2桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.prefecture">prefecture</strong></td>
<td valign="top"><a href="#prefecture">Prefecture</a></td>
<td>

市区町村の都道府県。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.wards">wards</strong></td>
<td valign="top">[<a href="#ward">Ward</a>!]!</td>
<td>

市区町村に属する区。政令指定都市の場合のみ存在します。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.datasets">datasets</strong></td>
<td valign="top">[<a href="#dataset">Dataset</a>!]!</td>
<td>

市区町村に属するデータセット（DatasetInput内のareasCodeの指定は無視されます）。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">input</td>
<td valign="top"><a href="#datasetsinput">DatasetsInput</a></td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.parentid">parentId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

地域の親となる地域のID。市区町村の親は都道府県です。政令指定都市の区の親は市です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.parent">parent</strong></td>
<td valign="top"><a href="#prefecture">Prefecture</a>!</td>
<td>

地域の親となる地域。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.planarcrsepsgcode">planarCrsEpsgCode</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

平面直角座標系のEPSGコード。例えば、東京都の場合は "6677" です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.citygmlid">citygmlId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

CityGMLデータセットのID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.citygml">citygml</strong></td>
<td valign="top"><a href="#citygmldataset">CityGMLDataset</a></td>
<td>

CityGMLデータセット。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="city.children">children</strong></td>
<td valign="top">[<a href="#area">Area</a>!]!</td>
<td>

地域に属する子地域。

</td>
</tr>
</tbody>
</table>

#### CityGMLDataset

PLATEAU標準製品仕様書に基づくCityGMLのデータセット。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.year">year</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの整備年度（西暦）。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.registrationyear">registrationYear</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの登録年度（西暦）。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.prefectureid">prefectureId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

データセットが属する都道府県のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.prefecturecode">prefectureCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a>!</td>
<td>

データセットが属する都道府県コード。2桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.cityid">cityId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

データセットが属する市のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.citycode">cityCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a>!</td>
<td>

データセットが属する市コード。先頭に都道府県コードを含む5桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.plateauspecminorid">plateauSpecMinorId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

データセットが準拠するPLATEAU都市モデルの仕様のマイナーバージョンへのID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.url">url</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

CityGMLのzip形式のファイルのURL。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.gspatialjpdataseturl">gspatialjpDatasetUrl</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

G空間情報センターへのURL。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.prefecture">prefecture</strong></td>
<td valign="top"><a href="#prefecture">Prefecture</a>!</td>
<td>

データセットが属する都道府県。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.city">city</strong></td>
<td valign="top"><a href="#city">City</a>!</td>
<td>

データセットが属する市。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.plateauspecminor">plateauSpecMinor</strong></td>
<td valign="top"><a href="#plateauspecminor">PlateauSpecMinor</a>!</td>
<td>

データセットが準拠するPLATEAU都市モデルの仕様。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.featuretypes">featureTypes</strong></td>
<td valign="top">[<a href="#string">String</a>!]!</td>
<td>

CityGMLが含む地物型コードのリスト。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.metadatazipurls">metadataZipUrls</strong></td>
<td valign="top">[<a href="#string">String</a>!]!</td>
<td>

CityGMLのメタデータを含むzipファイルURLのリスト。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="citygmldataset.admin">admin</strong></td>
<td valign="top"><a href="#any">Any</a></td>
<td>

管理者用

</td>
</tr>
</tbody>
</table>

#### GenericDataset

ユースケースデータなどを含む、その他のデータセット。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセット名

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.description">description</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

データセットの説明

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.year">year</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの整備年度（西暦）

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.registerationyear">registerationYear</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの公開年度（西暦）

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.groups">groups</strong></td>
<td valign="top">[<a href="#string">String</a>!]</td>
<td>

データセットを分類するグループ。グループが階層構造になっている場合は、親から子の順番で複数のグループ名が存在することがあります。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.opendataurl">openDataUrl</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

データセットの公開データのURL。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.prefectureid">prefectureId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

データセットが属する都道府県のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.prefecturecode">prefectureCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a></td>
<td>

データセットが属する都道府県コード。2桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.cityid">cityId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

データセットが属する市のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.citycode">cityCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a></td>
<td>

データセットが属する市コード。先頭に都道府県コードを含む5桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.wardid">wardId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

データセットが属する区のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.wardcode">wardCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a></td>
<td>

データセットが属する区コード。先頭に都道府県コードを含む5桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.typeid">typeId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

データセットの種類のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.typecode">typeCode</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットの種類コード。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.prefecture">prefecture</strong></td>
<td valign="top"><a href="#prefecture">Prefecture</a></td>
<td>

データセットが属する都道府県。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.city">city</strong></td>
<td valign="top"><a href="#city">City</a></td>
<td>

データセットが属する市。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.ward">ward</strong></td>
<td valign="top"><a href="#ward">Ward</a></td>
<td>

データセットが属する区。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.type">type</strong></td>
<td valign="top"><a href="#genericdatasettype">GenericDatasetType</a>!</td>
<td>

データセットの種類。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.items">items</strong></td>
<td valign="top">[<a href="#genericdatasetitem">GenericDatasetItem</a>!]!</td>
<td>

データセットのアイテム。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.ar">ar</strong></td>
<td valign="top"><a href="#boolean">Boolean</a>!</td>
<td>

PLATEAU ARで閲覧可能なデータセットかどうか。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdataset.admin">admin</strong></td>
<td valign="top"><a href="#any">Any</a></td>
<td>

管理者用

</td>
</tr>
</tbody>
</table>

#### GenericDatasetItem

その他のデータセットのアイテム。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="genericdatasetitem.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdatasetitem.format">format</strong></td>
<td valign="top"><a href="#datasetformat">DatasetFormat</a>!</td>
<td>

データセットのアイテムのフォーマット。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdatasetitem.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットのアイテム名。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdatasetitem.url">url</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットのアイテムのURL。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdatasetitem.layers">layers</strong></td>
<td valign="top">[<a href="#string">String</a>!]</td>
<td>

データセットのアイテムのレイヤー名。MVTやWMSなどのフォーマットの場合のみ存在。
レイヤー名が複数存在する場合は、同時に複数のレイヤーを表示可能であることを意味します。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdatasetitem.parentid">parentId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

データセットのアイテムが属するデータセットのID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdatasetitem.parent">parent</strong></td>
<td valign="top"><a href="#genericdataset">GenericDataset</a></td>
<td>

データセットのアイテムが属するデータセット。

</td>
</tr>
</tbody>
</table>

#### GenericDatasetType

その他のデータセットの種類。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="genericdatasettype.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdatasettype.code">code</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットの種類コード。「usecase」など。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdatasettype.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットの種類名。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdatasettype.category">category</strong></td>
<td valign="top"><a href="#datasettypecategory">DatasetTypeCategory</a>!</td>
<td>

データセットの種類のカテゴリ。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdatasettype.order">order</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの種類の順番を示す数字。大きいほど後に表示されます。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="genericdatasettype.datasets">datasets</strong></td>
<td valign="top">[<a href="#genericdataset">GenericDataset</a>!]!</td>
<td>

データセット（DatasetInput内のincludeTypesとexcludeTypesの指定は無視されます）。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">input</td>
<td valign="top"><a href="#datasetsinput">DatasetsInput</a></td>
<td></td>
</tr>
</tbody>
</table>

#### GlobalArea

全球（グローバル）エリア。特定の地域に属さない全球データを扱うための特殊なエリア。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="globalarea.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="globalarea.type">type</strong></td>
<td valign="top"><a href="#areatype">AreaType</a>!</td>
<td>

地域の種類

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="globalarea.code">code</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a>!</td>
<td>

地域コード。"global" という固定値。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="globalarea.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

地域名。"全球" という固定値。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="globalarea.datasets">datasets</strong></td>
<td valign="top">[<a href="#dataset">Dataset</a>!]!</td>
<td>

全球データセット（DatasetInput内のareasCodeの指定は無視されます）。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">input</td>
<td valign="top"><a href="#datasetsinput">DatasetsInput</a></td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="globalarea.parentid">parentId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

地域の親となる地域のID。GlobalAreaの場合は常にnull。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="globalarea.parent">parent</strong></td>
<td valign="top"><a href="#area">Area</a></td>
<td>

地域の親となる地域。GlobalAreaの場合は常にnull。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="globalarea.children">children</strong></td>
<td valign="top">[<a href="#area">Area</a>!]!</td>
<td>

地域に属する子地域。GlobalAreaの場合は常に空配列。

</td>
</tr>
</tbody>
</table>

#### PlateauDataset

PLATEAU都市モデルの通常のデータセット。例えば、地物型が建築物モデル（bldg）などのデータセットです。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセット名

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.subname">subname</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

データセットのサブ名。都市計画決定情報の○○区域や洪水浸水想定区域の河川名などが含まれます。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.subcode">subcode</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

データセットのサブコード。都市計画決定情報の○○区域や洪水浸水想定区域の河川名などのコード表現が含まれます。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.suborder">suborder</strong></td>
<td valign="top"><a href="#int">Int</a></td>
<td>

データセットのサブコードの順番。大きいほど後に表示されます。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.description">description</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

データセットの説明

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.year">year</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの整備年度（西暦）

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.registerationyear">registerationYear</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの公開年度（西暦）

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.groups">groups</strong></td>
<td valign="top">[<a href="#string">String</a>!]</td>
<td>

データセットを分類するグループ。グループが階層構造になっている場合は、親から子の順番で複数のグループ名が存在することがあります。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.opendataurl">openDataUrl</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

データセットの公開データのURL。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.prefectureid">prefectureId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

データセットが属する都道府県のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.prefecturecode">prefectureCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a></td>
<td>

データセットが属する都道府県コード。2桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.cityid">cityId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

データセットが属する市のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.citycode">cityCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a></td>
<td>

データセットが属する市コード。先頭に都道府県コードを含む5桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.wardid">wardId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

データセットが属する区のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.wardcode">wardCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a></td>
<td>

データセットが属する区コード。先頭に都道府県コードを含む5桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.typeid">typeId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

データセットの種類のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.typecode">typeCode</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットの種類コード。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.prefecture">prefecture</strong></td>
<td valign="top"><a href="#prefecture">Prefecture</a></td>
<td>

データセットが属する都道府県。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.city">city</strong></td>
<td valign="top"><a href="#city">City</a></td>
<td>

データセットが属する市。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.ward">ward</strong></td>
<td valign="top"><a href="#ward">Ward</a></td>
<td>

データセットが属する区。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.type">type</strong></td>
<td valign="top"><a href="#plateaudatasettype">PlateauDatasetType</a>!</td>
<td>

データセットの種類。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.items">items</strong></td>
<td valign="top">[<a href="#plateaudatasetitem">PlateauDatasetItem</a>!]!</td>
<td>

データセットのアイテム。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.ar">ar</strong></td>
<td valign="top"><a href="#boolean">Boolean</a>!</td>
<td>

PLATEAU ARで閲覧可能なデータセットかどうか。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.admin">admin</strong></td>
<td valign="top"><a href="#any">Any</a></td>
<td>

管理者用

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.plateauspecminorid">plateauSpecMinorId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

データセットが準拠するPLATEAU都市モデルの仕様のマイナーバージョンへのID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.plateauspecminor">plateauSpecMinor</strong></td>
<td valign="top"><a href="#plateauspecminor">PlateauSpecMinor</a>!</td>
<td>

データセットが準拠するPLATEAU都市モデルの仕様。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudataset.river">river</strong></td>
<td valign="top"><a href="#river">River</a></td>
<td>

河川。地物型が洪水浸水想定区域モデル（fld）の場合のみ存在します。

</td>
</tr>
</tbody>
</table>

#### PlateauDatasetItem

PLATEAU都市モデルのデータセットのアイテム。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasetitem.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasetitem.format">format</strong></td>
<td valign="top"><a href="#datasetformat">DatasetFormat</a>!</td>
<td>

データセットのアイテムのフォーマット。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasetitem.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットのアイテム名。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasetitem.url">url</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットのアイテムのURL。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasetitem.layers">layers</strong></td>
<td valign="top">[<a href="#string">String</a>!]</td>
<td>

データセットのアイテムのレイヤー名。MVTやWMSなどのフォーマットの場合のみ存在。
レイヤー名が複数存在する場合は、同時に複数のレイヤーを表示可能であることを意味します。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasetitem.parentid">parentId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

データセットのアイテムが属するデータセットのID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasetitem.parent">parent</strong></td>
<td valign="top"><a href="#plateaudataset">PlateauDataset</a></td>
<td>

データセットのアイテムが属するデータセット。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasetitem.lod">lod</strong></td>
<td valign="top"><a href="#int">Int</a></td>
<td>

データセットのアイテムのLOD（詳細度・Level of Detail）。1、2、3、4などの整数値です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasetitem.lodex">lodEx</strong></td>
<td valign="top"><a href="#int">Int</a></td>
<td>

データセットのアイテムのLOD（詳細度・Level of Detail）のうち、小数点以下の値が存在する場合に定義されます。例えばLOD3.1の場合は1、3.0の場合は0となります。LODがnullの場合はnullとなります。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasetitem.texture">texture</strong></td>
<td valign="top"><a href="#texture">Texture</a></td>
<td>

データセットのアイテムのテクスチャの種類。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasetitem.floodingscale">floodingScale</strong></td>
<td valign="top"><a href="#floodingscale">FloodingScale</a></td>
<td>

浸水規模。地物型が災害リスク（浸水）モデルの場合のみ存在することがあります。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasetitem.floodingscalesuffix">floodingScaleSuffix</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

浸水規模の枝番。地物型が災害リスク（浸水）モデルの場合のみ存在することがあります。

</td>
</tr>
</tbody>
</table>

#### PlateauDatasetType

PLATEAU都市モデルのデータセットの種類。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasettype.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasettype.code">code</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットの種類コード。「bldg」など。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasettype.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットの種類名。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasettype.category">category</strong></td>
<td valign="top"><a href="#datasettypecategory">DatasetTypeCategory</a>!</td>
<td>

データセットの種類のカテゴリ。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasettype.order">order</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの種類の順番を示す数字。大きいほど後に表示されます。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasettype.plateauspecid">plateauSpecId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

データセットの種類が属するPLATEAU都市モデルの仕様のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasettype.plateauspec">plateauSpec</strong></td>
<td valign="top"><a href="#plateauspec">PlateauSpec</a></td>
<td>

データセットの種類が属するPLATEAU都市モデルの仕様。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasettype.year">year</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの種類が属するPLATEAU都市モデルの仕様の公開年度（西暦）。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasettype.flood">flood</strong></td>
<td valign="top"><a href="#boolean">Boolean</a>!</td>
<td>

災害リスク（浸水）モデルかどうか。河川などの情報が利用可能です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateaudatasettype.datasets">datasets</strong></td>
<td valign="top">[<a href="#plateaudataset">PlateauDataset</a>!]!</td>
<td>

データセット（DatasetInput内のincludeTypesとexcludeTypesの指定は無視されます）。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">input</td>
<td valign="top"><a href="#datasetsinput">DatasetsInput</a></td>
<td></td>
</tr>
</tbody>
</table>

#### PlateauSpec

PLATEAU都市モデルの仕様のメジャーバージョン。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="plateauspec.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateauspec.majorversion">majorVersion</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

PLATEAU都市モデルの仕様のバージョン番号。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateauspec.year">year</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

仕様の公開年度（西暦）。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateauspec.datasettypes">datasetTypes</strong></td>
<td valign="top">[<a href="#plateaudatasettype">PlateauDatasetType</a>!]!</td>
<td>

その仕様に含まれるデータセットの種類。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateauspec.minorversions">minorVersions</strong></td>
<td valign="top">[<a href="#plateauspecminor">PlateauSpecMinor</a>!]!</td>
<td>

その仕様のマイナーバージョン。

</td>
</tr>
</tbody>
</table>

#### PlateauSpecMinor

PLATEAU都市モデルの仕様のマイナーバージョン。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="plateauspecminor.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateauspecminor.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

PLATEAU都市モデルの仕様の名前。 "第2.3版" のような文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateauspecminor.version">version</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

バージョンを表す文字列。 "2.3" のような文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateauspecminor.majorversion">majorVersion</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

メジャーバージョン番号。 2のような整数です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateauspecminor.year">year</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

仕様の公開年度（西暦）。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateauspecminor.parentid">parentId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

その仕様が属する仕様のメジャーバージョンのID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateauspecminor.parent">parent</strong></td>
<td valign="top"><a href="#plateauspec">PlateauSpec</a>!</td>
<td>

その仕様が属する仕様のメジャーバージョン。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="plateauspecminor.datasets">datasets</strong></td>
<td valign="top">[<a href="#dataset">Dataset</a>!]!</td>
<td>

その仕様に準拠して整備されたPLATEAU都市モデルデータセット（DatasetInput内のplateauSpecの指定は無視されます）。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">input</td>
<td valign="top"><a href="#datasetsinput">DatasetsInput</a></td>
<td></td>
</tr>
</tbody>
</table>

#### Prefecture

都道府県

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="prefecture.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="prefecture.type">type</strong></td>
<td valign="top"><a href="#areatype">AreaType</a>!</td>
<td>

地域の種類

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="prefecture.code">code</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a>!</td>
<td>

都道府県コード。2桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="prefecture.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

都道府県名

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="prefecture.cities">cities</strong></td>
<td valign="top">[<a href="#city">City</a>!]!</td>
<td>

都道府県に属する市区町村

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="prefecture.datasets">datasets</strong></td>
<td valign="top">[<a href="#dataset">Dataset</a>!]!</td>
<td>

都道府県に属するデータセット（DatasetInput内のareasCodeの指定は無視されます）。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">input</td>
<td valign="top"><a href="#datasetsinput">DatasetsInput</a></td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="prefecture.parentid">parentId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

地域の親となる地域のID。市区町村の親は都道府県です。政令指定都市の区の親は市です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="prefecture.parent">parent</strong></td>
<td valign="top"><a href="#area">Area</a></td>
<td>

地域の親となる地域。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="prefecture.children">children</strong></td>
<td valign="top">[<a href="#area">Area</a>!]!</td>
<td>

地域に属する子地域。

</td>
</tr>
</tbody>
</table>

#### RelatedDataset

PLATEAU都市モデルデータセットと併せて表示することで情報を補完できる、関連データセット。
避難施設・ランドマーク・鉄道駅・鉄道・緊急輸送道路・公園・行政界などのデータセット。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセット名

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.description">description</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

データセットの説明

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.year">year</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの整備年度（西暦）

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.registerationyear">registerationYear</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの公開年度（西暦）

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.groups">groups</strong></td>
<td valign="top">[<a href="#string">String</a>!]</td>
<td>

データセットを分類するグループ。グループが階層構造になっている場合は、親から子の順番で複数のグループ名が存在することがあります。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.opendataurl">openDataUrl</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

データセットの公開データのURL。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.prefectureid">prefectureId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

データセットが属する都道府県のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.prefecturecode">prefectureCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a></td>
<td>

データセットが属する都道府県コード。2桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.cityid">cityId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

データセットが属する市のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.citycode">cityCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a></td>
<td>

データセットが属する市コード。先頭に都道府県コードを含む5桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.wardid">wardId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

データセットが属する区のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.wardcode">wardCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a></td>
<td>

データセットが属する区コード。先頭に都道府県コードを含む5桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.typeid">typeId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

データセットの種類のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.typecode">typeCode</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットの種類コード。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.prefecture">prefecture</strong></td>
<td valign="top"><a href="#prefecture">Prefecture</a></td>
<td>

データセットが属する都道府県。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.city">city</strong></td>
<td valign="top"><a href="#city">City</a></td>
<td>

データセットが属する市。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.ward">ward</strong></td>
<td valign="top"><a href="#ward">Ward</a></td>
<td>

データセットが属する区。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.type">type</strong></td>
<td valign="top"><a href="#relateddatasettype">RelatedDatasetType</a>!</td>
<td>

データセットの種類。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.items">items</strong></td>
<td valign="top">[<a href="#relateddatasetitem">RelatedDatasetItem</a>!]!</td>
<td>

データセットのアイテム。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.ar">ar</strong></td>
<td valign="top"><a href="#boolean">Boolean</a>!</td>
<td>

PLATEAU ARで閲覧可能なデータセットかどうか。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddataset.admin">admin</strong></td>
<td valign="top"><a href="#any">Any</a></td>
<td>

管理者用

</td>
</tr>
</tbody>
</table>

#### RelatedDatasetItem

関連データセットのアイテム。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasetitem.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasetitem.format">format</strong></td>
<td valign="top"><a href="#datasetformat">DatasetFormat</a>!</td>
<td>

データセットのアイテムのフォーマット。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasetitem.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットのアイテム名。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasetitem.url">url</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットのアイテムのURL。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasetitem.originalformat">originalFormat</strong></td>
<td valign="top"><a href="#datasetformat">DatasetFormat</a></td>
<td>

データセットのアイテムの変換前データのフォーマット。
originalUrlフィールドが存在する場合のみ存在します。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasetitem.originalurl">originalUrl</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

データセットのアイテムの変換前データのURL。
鉄道駅情報・ランドマーク情報はurlフィールドではCZML形式で提供されていますが、元となったGeoJSONデータが存在します。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasetitem.layers">layers</strong></td>
<td valign="top">[<a href="#string">String</a>!]</td>
<td>

データセットのアイテムのレイヤー名。MVTやWMSなどのフォーマットの場合のみ存在。
レイヤー名が複数存在する場合は、同時に複数のレイヤーを表示可能であることを意味します。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasetitem.parentid">parentId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

データセットのアイテムが属するデータセットのID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasetitem.parent">parent</strong></td>
<td valign="top"><a href="#relateddataset">RelatedDataset</a></td>
<td>

データセットのアイテムが属するデータセット。

</td>
</tr>
</tbody>
</table>

#### RelatedDatasetType

関連データセットの種類。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasettype.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasettype.code">code</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットの種類コード。「park」など。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasettype.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットの種類名。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasettype.category">category</strong></td>
<td valign="top"><a href="#datasettypecategory">DatasetTypeCategory</a>!</td>
<td>

データセットの種類のカテゴリ。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasettype.order">order</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの種類の順番を示す数字。大きいほど後に表示されます。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="relateddatasettype.datasets">datasets</strong></td>
<td valign="top">[<a href="#relateddataset">RelatedDataset</a>!]!</td>
<td>

データセット（DatasetInput内のincludeTypesとexcludeTypesの指定は無視されます）。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">input</td>
<td valign="top"><a href="#datasetsinput">DatasetsInput</a></td>
<td></td>
</tr>
</tbody>
</table>

#### River

洪水浸水想定区域モデルにおける河川。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="river.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

河川名。通常、「〜水系〜川」という形式になります。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="river.admin">admin</strong></td>
<td valign="top"><a href="#riveradmin">RiverAdmin</a>!</td>
<td>

管理区間

</td>
</tr>
</tbody>
</table>

#### Ward

区（政令指定都市のみ）

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="ward.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="ward.type">type</strong></td>
<td valign="top"><a href="#areatype">AreaType</a>!</td>
<td>

種類

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="ward.code">code</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a>!</td>
<td>

区コード。先頭に都道府県コードを含む5桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="ward.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

区名

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="ward.prefectureid">prefectureId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

区が属する都道府県のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="ward.prefecturecode">prefectureCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a>!</td>
<td>

区が属する都道府県コード。2桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="ward.cityid">cityId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

区が属する市のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="ward.citycode">cityCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a>!</td>
<td>

区が属する市のコード。先頭に都道府県コードを含む5桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="ward.prefecture">prefecture</strong></td>
<td valign="top"><a href="#prefecture">Prefecture</a></td>
<td>

区が属する都道府県。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="ward.city">city</strong></td>
<td valign="top"><a href="#city">City</a></td>
<td>

区が属する市。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="ward.datasets">datasets</strong></td>
<td valign="top">[<a href="#dataset">Dataset</a>!]!</td>
<td>

区に属するデータセット（DatasetInput内のareasCodeの指定は無視されます）。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">input</td>
<td valign="top"><a href="#datasetsinput">DatasetsInput</a></td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="ward.parentid">parentId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

地域の親となる地域のID。市区町村の親は都道府県です。政令指定都市の区の親は市です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="ward.parent">parent</strong></td>
<td valign="top"><a href="#city">City</a>!</td>
<td>

地域の親となる地域。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="ward.children">children</strong></td>
<td valign="top">[<a href="#area">Area</a>!]!</td>
<td>

地域に属する子地域。

</td>
</tr>
</tbody>
</table>

### Inputs

#### AreasInput

地域を検索するためのクエリ。

<table>
<thead>
<tr>
<th colspan="2" align="left">Field</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="areasinput.parentcode">parentCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a></td>
<td>

検索したい地域が属する親となる地域のコード。例えば東京都に属する都市を検索したい場合は "13" を指定します。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="areasinput.datasettypes">datasetTypes</strong></td>
<td valign="top">[<a href="#string">String</a>!]</td>
<td>

データセットの種類コード。例えば、建築物モデルのデータセットが存在する地域を検索したい場合は "bldg" を指定します。複数指定するとOR条件で検索を行います。
未指定の場合、全てのデータセットの種類を対象に検索します。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="areasinput.categories">categories</strong></td>
<td valign="top">[<a href="#datasettypecategory">DatasetTypeCategory</a>!]</td>
<td>

データセットの種類のカテゴリ。例えば、PLATEAU都市モデルデータセットが存在する地域を検索したい場合は PLATEAU を指定します。複数指定するとOR条件で検索を行います。
未指定の場合、全てのカテゴリのデータセットを対象に検索します。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="areasinput.areatypes">areaTypes</strong></td>
<td valign="top">[<a href="#areatype">AreaType</a>!]</td>
<td>

地域の種類。例えば、市を検索したい場合は CITY を指定します。複数指定するとOR条件で検索を行います。
未指定の場合、全ての地域を対象に検索します。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="areasinput.searchtokens">searchTokens</strong></td>
<td valign="top">[<a href="#string">String</a>!]</td>
<td>

検索文字列。複数指定するとAND条件で絞り込み検索が行えます。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="areasinput.includeparents">includeParents</strong></td>
<td valign="top"><a href="#boolean">Boolean</a></td>
<td>

datasetTypes が指定された場合に、検索結果にその地域の親も含めるかどうか。デフォルトは false です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="areasinput.includeempty">includeEmpty</strong></td>
<td valign="top"><a href="#boolean">Boolean</a></td>
<td>

属しているDatasetが存在しない都市を含めます。通常のデータセットは存在しないが、 CityGMLDataset の city として使用されている都市が含まれます。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="areasinput.deep">deep</strong></td>
<td valign="top"><a href="#boolean">Boolean</a></td>
<td>

parentCode が指定された場合に、その地域に間接的に属している地域も検索対象にするかどうか。デフォルトは false です。

</td>
</tr>
</tbody>
</table>

#### DatasetTypesInput

データセットの種類を検索するためのクエリ。

<table>
<thead>
<tr>
<th colspan="2" align="left">Field</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="datasettypesinput.category">category</strong></td>
<td valign="top"><a href="#datasettypecategory">DatasetTypeCategory</a></td>
<td>

データセットの種類のカテゴリ。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasettypesinput.plateauspec">plateauSpec</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

データセットの種類が属するPLATEAU都市モデルの仕様名。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasettypesinput.year">year</strong></td>
<td valign="top"><a href="#int">Int</a></td>
<td>

データセットの種類が属するPLATEAU都市モデルの仕様の公開年度（西暦）。

</td>
</tr>
</tbody>
</table>

#### DatasetsInput

データセットを検索するためのクエリ。

<table>
<thead>
<tr>
<th colspan="2" align="left">Field</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="datasetsinput.areacodes">areaCodes</strong></td>
<td valign="top">[<a href="#areacode">AreaCode</a>!]</td>
<td>

データセットの地域コード（都道府県コードや市区町村コードが使用可能）。複数指定するとOR条件で検索を行います。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetsinput.plateauspec">plateauSpec</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

仕様書のバージョン。「第2.3版」「2.3」「2」などの文字列が使用可能です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetsinput.year">year</strong></td>
<td valign="top"><a href="#int">Int</a></td>
<td>

データの整備年度（西暦）。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetsinput.registrationyear">registrationYear</strong></td>
<td valign="top"><a href="#int">Int</a></td>
<td>

データの公開年度（西暦）。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetsinput.excludetypes">excludeTypes</strong></td>
<td valign="top">[<a href="#string">String</a>!]</td>
<td>

検索結果から除外するデータセットの種類コード。種類コードは例えば "bldg"（建築物モデル）の他、"plateau"（PLATEAU都市モデルデータセット）、"related"（関連データセット）、"generic"（その他のデータセット）が使用可能です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetsinput.includetypes">includeTypes</strong></td>
<td valign="top">[<a href="#string">String</a>!]</td>
<td>

検索結果に含めるデータセットの種類コード。未指定の場合、全てのデータセットの種類を対象に検索し、指定するとその種類で検索結果を絞り込みます。種類コードは例えば "bldg"（建築物モデル）の他、"plateau"（PLATEAU都市モデルデータセット）、"related"（関連データセット）、"generic"（その他のデータセット）が使用可能です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetsinput.searchtokens">searchTokens</strong></td>
<td valign="top">[<a href="#string">String</a>!]</td>
<td>

検索文字列。複数指定するとAND条件で絞り込み検索が行えます。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetsinput.shallow">shallow</strong></td>
<td valign="top"><a href="#boolean">Boolean</a></td>
<td>

areaCodesで指定された地域に直接属しているデータセットのみを検索対象にするかどうか。
デフォルトはfalseで、指定された地域に間接的に属するデータセットも全て検索します。
例えば、札幌市を対象にした場合、札幌市には中央区や北区といった区のデータセットも存在しますが、trueにすると札幌市のデータセットのみを返します。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetsinput.groupedonly">groupedOnly</strong></td>
<td valign="top"><a href="#boolean">Boolean</a></td>
<td>

特殊なグループを持つデータセットのみを検索対象にするかどうか。デフォルトはfalseです。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetsinput.ar">ar</strong></td>
<td valign="top"><a href="#boolean">Boolean</a></td>
<td>

PLATEAU ARで閲覧可能なデータセットを含めるかどうか。
trueの場合はARで閲覧可能なデータセットのみ、falseの場合はARで閲覧不可能なデータセットのみを返します。

</td>
</tr>
</tbody>
</table>

### Enums

#### AreaType

<table>
<thead>
<tr>
<th align="left">Value</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td valign="top"><strong>PREFECTURE</strong></td>
<td>

都道府県

</td>
</tr>
<tr>
<td valign="top"><strong>CITY</strong></td>
<td>

市町村

</td>
</tr>
<tr>
<td valign="top"><strong>WARD</strong></td>
<td>

区（政令指定都市のみ）

</td>
</tr>
<tr>
<td valign="top"><strong>GLOBAL</strong></td>
<td>

全球（グローバル）

</td>
</tr>
</tbody>
</table>

#### DatasetFormat

データセットのフォーマット。

<table>
<thead>
<tr>
<th align="left">Value</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td valign="top"><strong>CSV</strong></td>
<td>

CSV

</td>
</tr>
<tr>
<td valign="top"><strong>CZML</strong></td>
<td>

CZML

</td>
</tr>
<tr>
<td valign="top"><strong>CESIUM3DTILES</strong></td>
<td>

3D Tiles

</td>
</tr>
<tr>
<td valign="top"><strong>GLTF</strong></td>
<td>

GlTF

</td>
</tr>
<tr>
<td valign="top"><strong>GTFS_REALTIME</strong></td>
<td>

GTFS Realtime

</td>
</tr>
<tr>
<td valign="top"><strong>GEOJSON</strong></td>
<td>

GeoJSON

</td>
</tr>
<tr>
<td valign="top"><strong>MVT</strong></td>
<td>

Mapbox Vector Tile

</td>
</tr>
<tr>
<td valign="top"><strong>TMS</strong></td>
<td>

Tile Map Service

</td>
</tr>
<tr>
<td valign="top"><strong>TILES</strong></td>
<td>

XYZで分割された画像タイル。/{z}/{x}/{y}.png のようなURLになります。

</td>
</tr>
<tr>
<td valign="top"><strong>WMS</strong></td>
<td>

Web Map Service

</td>
</tr>
</tbody>
</table>

#### DatasetTypeCategory

データセットの種類のカテゴリ。

<table>
<thead>
<tr>
<th align="left">Value</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td valign="top"><strong>PLATEAU</strong></td>
<td>

PLATEAU都市モデルデータセット

</td>
</tr>
<tr>
<td valign="top"><strong>RELATED</strong></td>
<td>

関連データセット

</td>
</tr>
<tr>
<td valign="top"><strong>GENERIC</strong></td>
<td>

その他のデータセット

</td>
</tr>
</tbody>
</table>

#### FloodingScale

浸水想定区域モデルにおける浸水規模。

<table>
<thead>
<tr>
<th align="left">Value</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td valign="top"><strong>PLANNED</strong></td>
<td>

計画規模

</td>
</tr>
<tr>
<td valign="top"><strong>EXPECTED_MAXIMUM</strong></td>
<td>

想定最大規模

</td>
</tr>
</tbody>
</table>

#### RiverAdmin

河川の管理区間

<table>
<thead>
<tr>
<th align="left">Value</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td valign="top"><strong>NATIONAL</strong></td>
<td>

国管理区間

</td>
</tr>
<tr>
<td valign="top"><strong>PREFECTURE</strong></td>
<td>

都道府県管理区間

</td>
</tr>
</tbody>
</table>

#### Texture

建築物モデルのテクスチャの種類。

<table>
<thead>
<tr>
<th align="left">Value</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td valign="top"><strong>NONE</strong></td>
<td>

テクスチャなし

</td>
</tr>
<tr>
<td valign="top"><strong>TEXTURE</strong></td>
<td>

テクスチャあり

</td>
</tr>
</tbody>
</table>

### Scalars

#### Any

#### AreaCode

行政コードを表す文字列。
都道府県の場合は、2桁の数字で構成された文字列です。
自治体の場合は、先頭に2桁の都道府県コードを含む、5桁の数字で構成された文字列です。

#### Boolean

The `Boolean` scalar type represents `true` or `false`.

#### ID

The `ID` scalar type represents a unique identifier, often used to refetch an object or as key for a cache. The ID type appears in a JSON response as a String; however, it is not intended to be human-readable. When expected as an input type, any string (such as `"4"`) or integer (such as `4`) input value will be accepted as an ID.

#### Int

The `Int` scalar type represents non-fractional signed whole numeric values. Int can represent values between -(2^31) and 2^31 - 1.

#### String

The `String` scalar type represents textual data, represented as UTF-8 character sequences. The String type is most often used by GraphQL to represent free-form human-readable text.


### Interfaces


#### Area

地域。都道府県（Prefecture）・市区町村（City）・区（政令指定都市のみ・Ward）のいずれかです。
政令指定都市の場合のみ、市の下に区が存在します。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="area.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="area.type">type</strong></td>
<td valign="top"><a href="#areatype">AreaType</a>!</td>
<td>

地域の種類

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="area.code">code</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a>!</td>
<td>

地域コード。行政コードや市区町村コードとも呼ばれます。
都道府県の場合は二桁の数字から成る文字列です。
市区町村の場合は、先頭に都道府県コードを含む5桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="area.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

地域名

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="area.datasets">datasets</strong></td>
<td valign="top">[<a href="#dataset">Dataset</a>!]!</td>
<td>

地域に属するデータセット（DatasetInput内のareasCodeの指定は無視されます）。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">input</td>
<td valign="top"><a href="#datasetsinput">DatasetsInput</a></td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="area.parentid">parentId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

地域の親となる地域のID。市区町村の親は都道府県です。政令指定都市の区の親は市です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="area.parent">parent</strong></td>
<td valign="top"><a href="#area">Area</a></td>
<td>

地域の親となる地域。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="area.children">children</strong></td>
<td valign="top">[<a href="#area">Area</a>!]!</td>
<td>

地域に属する子地域。

</td>
</tr>
</tbody>
</table>

**Possible Types:** [City](#city), [GlobalArea](#globalarea), [Prefecture](#prefecture), [Ward](#ward)

#### Dataset

データセット。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="dataset.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセット名

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.description">description</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

データセットの説明

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.year">year</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの整備年度（西暦）

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.registerationyear">registerationYear</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの登録年度（西暦）

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.groups">groups</strong></td>
<td valign="top">[<a href="#string">String</a>!]</td>
<td>

データセットを分類するグループ。グループが階層構造になっている場合は、親から子の順番で複数のグループ名が存在することがあります。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.opendataurl">openDataUrl</strong></td>
<td valign="top"><a href="#string">String</a></td>
<td>

データセットの公開データのURL。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.prefectureid">prefectureId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

データセットが属する都道府県のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.prefecturecode">prefectureCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a></td>
<td>

データセットが属する都道府県コード。2桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.cityid">cityId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

データセットが属する市のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.citycode">cityCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a></td>
<td>

データセットが属する市コード。先頭に都道府県コードを含む5桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.wardid">wardId</strong></td>
<td valign="top"><a href="#id">ID</a></td>
<td>

データセットが属する区のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.wardcode">wardCode</strong></td>
<td valign="top"><a href="#areacode">AreaCode</a></td>
<td>

データセットが属する区コード。先頭に都道府県コードを含む5桁の数字から成る文字列です。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.typeid">typeId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

データセットの種類のID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.typecode">typeCode</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットの種類コード。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.prefecture">prefecture</strong></td>
<td valign="top"><a href="#prefecture">Prefecture</a></td>
<td>

データセットが属する都道府県。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.city">city</strong></td>
<td valign="top"><a href="#city">City</a></td>
<td>

データセットが属する市。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.ward">ward</strong></td>
<td valign="top"><a href="#ward">Ward</a></td>
<td>

データセットが属する区。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.type">type</strong></td>
<td valign="top"><a href="#datasettype">DatasetType</a>!</td>
<td>

データセットの種類。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.items">items</strong></td>
<td valign="top">[<a href="#datasetitem">DatasetItem</a>!]!</td>
<td>

データセットのアイテム。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.ar">ar</strong></td>
<td valign="top"><a href="#boolean">Boolean</a>!</td>
<td>

PLATEAU ARで閲覧可能なデータセットかどうか。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="dataset.admin">admin</strong></td>
<td valign="top"><a href="#any">Any</a></td>
<td>

管理者用

</td>
</tr>
</tbody>
</table>

**Possible Types:** [GenericDataset](#genericdataset), [PlateauDataset](#plateaudataset), [RelatedDataset](#relateddataset)

#### DatasetItem

データセットのアイテム。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="datasetitem.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetitem.format">format</strong></td>
<td valign="top"><a href="#datasetformat">DatasetFormat</a>!</td>
<td>

データセットのアイテムのフォーマット。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetitem.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットのアイテム名。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetitem.url">url</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットのアイテムのURL。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetitem.layers">layers</strong></td>
<td valign="top">[<a href="#string">String</a>!]</td>
<td>

データセットのアイテムのレイヤー名。MVTやWMSなどのフォーマットの場合のみ存在。
レイヤー名が複数存在する場合は、同時に複数のレイヤーを表示可能であることを意味します。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetitem.parentid">parentId</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

データセットのアイテムが属するデータセットのID。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasetitem.parent">parent</strong></td>
<td valign="top"><a href="#dataset">Dataset</a></td>
<td>

データセットのアイテムが属するデータセット。

</td>
</tr>
</tbody>
</table>

**Possible Types:** [GenericDatasetItem](#genericdatasetitem), [PlateauDatasetItem](#plateaudatasetitem), [RelatedDatasetItem](#relateddatasetitem)

#### DatasetType

データセットの種類。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="datasettype.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td></td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasettype.code">code</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットの種類コード。 "bldg" など。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasettype.name">name</strong></td>
<td valign="top"><a href="#string">String</a>!</td>
<td>

データセットの種類名。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasettype.category">category</strong></td>
<td valign="top"><a href="#datasettypecategory">DatasetTypeCategory</a>!</td>
<td>

データセットの種類のカテゴリ。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasettype.order">order</strong></td>
<td valign="top"><a href="#int">Int</a>!</td>
<td>

データセットの種類の順番を示す数字。大きいほど後に表示されます。

</td>
</tr>
<tr>
<td colspan="2" valign="top"><strong id="datasettype.datasets">datasets</strong></td>
<td valign="top">[<a href="#dataset">Dataset</a>!]!</td>
<td>

データセット（DatasetInput内のincludeTypesとexcludeTypesの指定は無視されます）。

</td>
</tr>
<tr>
<td colspan="2" align="right" valign="top">input</td>
<td valign="top"><a href="#datasetsinput">DatasetsInput</a></td>
<td></td>
</tr>
</tbody>
</table>

**Possible Types:** [GenericDatasetType](#genericdatasettype), [PlateauDatasetType](#plateaudatasettype), [RelatedDatasetType](#relateddatasettype)

#### Node

IDを持つオブジェクト。nodeまたはnodesクエリでIDを指定して検索可能です。

<table>
<thead>
<tr>
<th align="left">Field</th>
<th align="right">Argument</th>
<th align="left">Type</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td colspan="2" valign="top"><strong id="node.id">id</strong></td>
<td valign="top"><a href="#id">ID</a>!</td>
<td>

オブジェクトのID

</td>
</tr>
</tbody>
</table>

**Possible Types:** [City](#city), [CityGMLDataset](#citygmldataset), [GenericDataset](#genericdataset), [GenericDatasetItem](#genericdatasetitem), [GenericDatasetType](#genericdatasettype), [GlobalArea](#globalarea), [PlateauDataset](#plateaudataset), [PlateauDatasetItem](#plateaudatasetitem), [PlateauDatasetType](#plateaudatasettype), [PlateauSpec](#plateauspec), [PlateauSpecMinor](#plateauspecminor), [Prefecture](#prefecture), [RelatedDataset](#relateddataset), [RelatedDatasetItem](#relateddatasetitem), [RelatedDatasetType](#relateddatasettype), [Ward](#ward)
