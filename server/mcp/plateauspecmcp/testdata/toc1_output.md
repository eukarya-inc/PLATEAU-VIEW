# 1  概覧

## 1 概覧

## 1.1 製品仕様の作成情報

| 製品仕様の題名 | 3D都市モデル標準製品仕様書 第5.0版 |
| --- | --- |
| 日付 | 2025/03/21 |
| 作成者 | 国土交通省 |
| 言語 | 日本語 |
| 分野 | 都市 |
| 文書書式 | PDF/WORD |

## 1.2 目的

「3D都市モデル標準製品仕様書」（以下、「標準製品仕様書」と呼ぶ）は、 各都市において3D都市モデルを整備する際にその製品仕様を適切に作成でき、かつ、各都市の製品仕様に従って整備された3D都市モデルが国際標準に準拠したものになることを目的として提供する標準文書である。

標準製品仕様書では、以下に示すユースケースに3D都市モデルが使用されることを想定し、その製品仕様を定めている。

- 都市に関わる様々な地理空間データを格納する基盤（オープンデータ化を含む）
- 3次元空間における都市計画決定情報の可視化
- 災害リスク情報の3次元可視化

各都市の3D都市モデルを整備する際には、あらかじめ標準製品仕様書を参照し、各都市の製品仕様を決定し、都市ごとの製品仕様書（以下、「拡張製品仕様書」と呼ぶ）を作成しなければならない。

3D都市モデルのユースケースによって必要な地物（現実世界の現象を抽象化した概念）やその属性（地物の特性）は異なる。また、対象となる都市の環境により、同じユースケースであっても必要な地物等が異なる場合がある。

そこで、標準製品仕様書では、標準製品仕様書から、3D都市モデルに含めたい地物やその属性を抽出したり、不足する地物や属性を追加したりするための規則を定めている。この規則に従い、製品仕様を決定することで、それぞれの拡張製品仕様書を国際標準に準拠しつつ作成できる（図1-1）。

各都市での拡張製品仕様書の作成に必要なテンプレートの一式は、3D都市モデル標準製品仕様書HTML版ウェブページ（ [https://www.mlit.go.jp/plateaudocument/](https://www.mlit.go.jp/plateaudocument/) ）より入手できる。

*[Image]*

## 1.3 製品の範囲

標準製品仕様書に基づくデータ製品の空間範囲は日本国内全域を含む範囲とする。標準製品仕様書に基づくデータ製品の時間範囲は任意であり、特に定めない。

各都市の拡張製品仕様書において、データ製品の範囲を指定する。なお、拡張製品仕様書におけるデータ製品の空間範囲は、基礎自治体を原則とする。ただし、複数の基礎自治体に跨って存在する地物をデータ製品に含める場合にはこの限りではない。

## 1.4 引用規格等

本書が引用する規格・仕様・マニュアルを以下に示す。3D都市モデルの整備に際しては、これらの文献を理解することが前提となる。

引用文献のうちで、版の記載があるものは、その版を適用し、その後の改正版（追補を含む）は適用しない。版の記載がないものは、その最新版（追補を含む）を適用する。

PLATEAU Handbook #03, 国土交通省都市局. 3D都市モデル整備のためのBIM活用マニュアル. Version 第3.0版．国土交通省．入手先: [https://www.mlit.go.jp/plateau/libraries/handbooks/](https://www.mlit.go.jp/plateau/libraries/handbooks/) [[citation]](https://www.mlit.go.jp/plateau/libraries/handbooks/)ISO 19106:2004, Geographic information — Profiles [[src]](https://www.iso.org/standard/26011.html) [[obp]](https://www.iso.org/obp/ui/en/#!iso:std:26011:en) [[rss]](https://www.iso.org/contents/data/standard/02/60/26011.detail.rss)ISO/IEC 19501:2005 / iso-reference ISO/IEC 19501:2005(E) / URN urn:iso:std:iso-iec:19501:stage-90.60, Information technology — Open Distributed Processing — Unified Modeling Language (UML) Version 1.4.2 [[src]](https://www.iso.org/standard/32620.html) [[obp]](https://www.iso.org/obp/ui/en/#!iso:std:32620:en) [[rss]](https://www.iso.org/contents/data/standard/03/26/32620.detail.rss) [[citation]](https://www.iso.org/standard/32620.html)JPGIS 品質の要求，評価及び報告のための規則, 国土交通省国土地理院．品質の要求，評価及び報告のための規則．入手先: [https://www.gsi.go.jp/GIS/jpgis-downloads.html](https://www.gsi.go.jp/GIS/jpgis-downloads.html) [[citation]](https://www.gsi.go.jp/GIS/jpgis-downloads.html)作業規程の準則について, 国土交通省国土地理院．作業規程の準則．入手先: [https://www.gsi.go.jp/gijyutukanri/gijyutukanri41018.html](https://www.gsi.go.jp/gijyutukanri/gijyutukanri41018.html) [[citation]](https://www.gsi.go.jp/gijyutukanri/gijyutukanri41018.html)河川基盤地図ガイドライン（案）, 国土交通省都市局．河川基盤地図. Version 第2.1版．入手先: [https://www.mlit.go.jp/river/shishin_guideline/kasen/gis/pdf_docs/chizu/kiban_chizu.pdf](https://www.mlit.go.jp/river/shishin_guideline/kasen/gis/pdf_docs/chizu/kiban_chizu.pdf) [[citation]](https://www.mlit.go.jp/river/shishin_guideline/kasen/gis/pdf_docs/chizu/kiban_chizu.pdf)道路基盤地図情報（整備促進版）製品仕様書（案）, 国土交通省国土技術政策総合研究所．道路基盤地図情報．入手先: [https://www.nilim.go.jp/lab/bcg/siryou/tnn/tnn0848pdf/ks084811.pdf](https://www.nilim.go.jp/lab/bcg/siryou/tnn/tnn0848pdf/ks084811.pdf) [[citation]](https://www.nilim.go.jp/lab/bcg/siryou/tnn/tnn0848pdf/ks084811.pdf)都市計画データ標準製品仕様書, 国土交通省都市局．都市計画データ標準製品仕様書．入手先: [https://www.mlit.go.jp/toshi/tosiko/toshi_tosiko_tk_000187.html](https://www.mlit.go.jp/toshi/tosiko/toshi_tosiko_tk_000187.html) [[citation]](https://www.mlit.go.jp/toshi/tosiko/toshi_tosiko_tk_000187.html)電子国土基本図地図情報ファイル仕様書, 国土交通省国土地理院．電子国土基本図地図. Version 第1.1版．入手先: [https://www.gsi.go.jp/common/000189294.pdf](https://www.gsi.go.jp/common/000189294.pdf) [[citation]](https://www.gsi.go.jp/common/000189294.pdf)高潮浸水想定区域図作成の手引き, 国土交通省国土地理院．高潮浸水想定区域. Version 第2.11版．入手先: [https://www.mlit.go.jp/river/shishin_guideline/kaigan/takashioshinsui_manual.pdf](https://www.mlit.go.jp/river/shishin_guideline/kaigan/takashioshinsui_manual.pdf) [[citation]](https://www.mlit.go.jp/river/shishin_guideline/kaigan/takashioshinsui_manual.pdf)JPGIS 解説書 1.0, 国土交通省国土地理院．地理情報標準プロファイル（JPGIS） Ver. 1.0　解説書．入手先: [https://www.gsi.go.jp/GIS/jpgis-rh.html](https://www.gsi.go.jp/GIS/jpgis-rh.html) [[citation]](https://www.gsi.go.jp/GIS/jpgis-rh.html)IFC2x3 CV2.0, buildingSMART International. IFC 2×3 Coordination View 2.0. 入手先: [https://standards.buildingsmart.org/IFC/RELEASE/IFC2x3/FINAL/HTML/](https://standards.buildingsmart.org/IFC/RELEASE/IFC2x3/FINAL/HTML/) [[citation]](https://standards.buildingsmart.org/IFC/RELEASE/IFC2x3/FINAL/HTML/)JMP 2.0, 国土交通省国土地理院. JMP 2.0仕様書．入手先: [https://www.gsi.go.jp/GIS/jpgis-downloads.html](https://www.gsi.go.jp/GIS/jpgis-downloads.html) [[citation]](https://www.gsi.go.jp/GIS/jpgis-downloads.html)3次元屋内地理空間情報データ仕様書（案）, 国土交通省国土地理院. 3次元屋内地理空間情報データ仕様書．入手先: [https://www.gsi.go.jp/common/000212582.pdf](https://www.gsi.go.jp/common/000212582.pdf) [[citation]](https://www.gsi.go.jp/common/000212582.pdf)i-Construction推進のための3次元数値地形図データ作成マニュアル, 国土交通省国土地理院. i-Construction推進．入手先: [https://www.gsi.go.jp/gijyutukanri/gijyutukanri41029.html](https://www.gsi.go.jp/gijyutukanri/gijyutukanri41029.html) [[citation]](https://www.gsi.go.jp/gijyutukanri/gijyutukanri41029.html)i-UR 3.2, 内閣府地方創生推進事務局. Data Encoding Specification of i-Urban Revitalization -Urban Planning ADE- ver.3.2. 入手先: [https://www.chisou.go.jp/tiiki/toshisaisei/itoshisaisei/iur/index.html](https://www.chisou.go.jp/tiiki/toshisaisei/itoshisaisei/iur/index.html) [[citation]](https://www.chisou.go.jp/tiiki/toshisaisei/itoshisaisei/iur/index.html)JPGIS 2014, 国土交通省国土地理院．地理情報標準プロファイル（JPGIS）2014. 入手先: [https://www.gsi.go.jp/GIS/jpgis-downloads.html](https://www.gsi.go.jp/GIS/jpgis-downloads.html) [[citation]](https://www.gsi.go.jp/GIS/jpgis-downloads.html)地図情報レベル 2500 数値地形図データ作成のための標準製品仕様書（案）, 国土交通省国土地理院．地図情報レベル 2500数値地形図データ標準製品仕様書．入手先: [https://www.gsi.go.jp/common/000259076.pdf](https://www.gsi.go.jp/common/000259076.pdf) [[citation]](https://www.gsi.go.jp/common/000259076.pdf)OGC 12-019, Gerhard Gröger, Thomas H. Kolbe, Claus Nagel,  Karl-Heinz Häfele. OGC City Geography Markup Language (CityGML) Encoding Standard. Open Geospatial Consortium. 入手先: [https://www.ogc.org/standards/citygml](https://www.ogc.org/standards/citygml) [[src]](http://www.opengis.net/spec/citygml/2.0) [[pdf]](https://portal.ogc.org/files/?artifact_id=47842) [[citation]](https://www.ogc.org/standards/citygml)OGC 05-099r2, Arliss Whiteside. GML 3.1.1 simple dictionary profile. Version 2. Open Geospatial Consortium. 入手先: [https://www.ogc.org/standards/gml](https://www.ogc.org/standards/gml) [[pdf]](https://portal.ogc.org/files/?artifact_id=13206) [[citation]](https://www.ogc.org/standards/gml)PLATEAU Handbook #03-1, 国土交通省都市局. 3D都市モデル整備のためのBIM活用マニュアル（第3.0版）（別冊）3D都市モデルとの連携のためのBIMモデルIDM・MVD. Version 第2.0版．入手先: [https://www.mlit.go.jp/plateau/libraries/handbooks/](https://www.mlit.go.jp/plateau/libraries/handbooks/) [[citation]](https://www.mlit.go.jp/plateau/libraries/handbooks/)## 1.5 用語と定義

標準製品仕様書で使用する用語を示す。以下に記載のない用語とその定義については、
 JPGIS 2014付属書5（規定）定義に従う。


**1.5.1 3D都市モデル**
都市空間の地物及び属性を都市スケールで3次元的に再現したCityGML形式のデータ。

**1.5.2 BIM（Building Information Modeling）**
コンピュータ上に作成した主に三次元の形状情報に加え、室等の名称・⾯積、材料・部材の仕様・性能、仕上げ等、建築物の属性情報を併せ持つ建築物情報モデルを構築するもの。［出典： PLATEAU Handbook #03-1]

**1.5.3 BIM モデル**
コンピュータ上に作成した三次元の形状情報に加え、室等の名称・⾯積、材料・部材の仕様・性能、仕上げ等の建築物の属性情報を併せ持つ建築物情報モデル。［出典： PLATEAU Handbook #03-1]

**1.5.4 IFC（Industry Foundation Classes）**
buildingSMART International(以降bSI)が策定した三次元モデルデータ形式。 2013年にはISO 16739:2013:Ver.4.0.0.0(IFC4)として、国際標準として承認されている。  2018年に改訂され、ISO 16739-1:2018:Ver.4.0.2.1(IFC4 ADD2 TC1)が最新である。当初は、建築分野でのデータ交換を対象にしていたが、2013年にはbSI内にInfrastructure Roomが設置され、⼟⽊分野を対象にした検討が進められている。［出典： PLATEAU Handbook #03-1]

**1.5.5 Levels Of Detail（LOD）**
詳細さの度合い（詳細度）であり、CityGMLにおいて定義されている一つのオブジェクトの幾何を、その利用や可視化の目的に応じて、複数の段階に抽象化することを可能とする、マルチスケールなモデリングの仕組みである。［出典： OGC 12-019]

**1.5.6 応用スキーマ**
一つ又は複数の応用システムによって要求されるデータのための概念スキーマ。［出典： JPGIS 2014]

**1.5.7 数値地形図**
都市、河川、道路、ダム等の計画、管理及び土木工事のために使用できる位置精度を有した地理空間情報及び数値地形図。［出典: 付録７公共測量標準図式］

**1.5.8 地物**
現実世界の現象の抽象概念。地物は型又はインスタンスとして存在できる。地物型又は地物インスタンスはいずれか一方を意味する場合に用いるべきである。［出典： JPGIS 2014]

**1.5.9 地物属性**
地物の特性。［出典： JPGIS 2014]

**1.5.10 地物関連**
地物間の関係。［出典： JPGIS 2014]

**1.5.11 関連役割**
関連において相手の地物に対する自分の役割を指す。［出典： JPGIS 解説書 1.0]

**1.5.12 プロファイル**
1つ以上の基本規格のセット又は基本規格のサブセット及び該当する場合には特定の機能を達成するために必要なそれらの基本規格から選択された条項、クラス、オプション及びパラメータの識別。注記11.5.12の注記1標準製品仕様書は、i-UR及びCityGMLから3D都市モデルとして必要な地物型等をi-UR及びCityGMLと矛盾なく抽出した、i-UR及びCityGMLのプロファイルである。また、各都市で作成される拡張製品仕様書も、i-UR及びCityGMLのプロファイルでなくてはならない。

## 1.6 記号及び略語

BIM

Building Information Modeling

CityGML

City Geography Markup Language

GML

Geography Markup Language

IDM

Information Delivery Manual

IFC

Industry Foundation Classes

i-UR

Data Encoding Specification of i-Urban Revitalization -Urban Planning ADE-

JPGIS

Japan Profile of Geographic Information Standards

LOD

Level Of Detail

MMS

Mobile Mapping System

MVD

Model View Definition

UML

Unified Modeling Language

なお、標準製品仕様書で使用する以下の略語は、特段の記載がない場合にはそれぞれ下表に示す版を指す。

| 略語 | 使用する版 | 備考 |
| --- | --- | --- |
| CityGML | CityGML2.0 | 　 |
| GML | GML3.1.1 | ISO19136に対応するGMLの版はGML3.2.1であるが、CityGML2.0が参照するGMLの版は、GML3.1.1である。そのため、GML3.2.1と矛盾のない範囲でGML3.1.1を使用する。 |
| i-UR | i-UR3.2 | 　 |

## 1.7 拡張製品仕様書作成のためのテンプレートの入手方法

標準製品仕様書に基づき、各都市でそれぞれの拡張製品仕様書を作成する際に必要となるテンプレート等のファイルを一式にまとめたものを、3D都市モデル標準製品仕様書HTML版ウェブページ
 （https://www.mlit.go.jp/plateaudocument/ ）より入手できる。

テンプレート一式には、以下を含む。

|  | テンプレート一式の内容 | ファイル名 | 備考 |
| --- | --- | --- | --- |
| 1 | 拡張製品仕様書テンプレート | template_specification.docx | 標準製品仕様書のWORDファイルに、拡張製品仕様書で追記すべき箇所を示したもの。 |
| 2 | 取得項目一覧 |  | シート名：A.3.1_取得項目一覧 |
| 3 | 拡張製品仕様書：拡張コードリスト |  | シート名：A.3.2_拡張コードリスト |
| 4 | 拡張製品仕様書：建築物の拡張属性 |  | シート名：A.3.3_建築物の拡張属性リスト |
| 5 | 拡張製品仕様書：拡張地物定義 |  | シート名：A.3.4_拡張地物定義 |
| 6 | 拡張製品仕様書：汎用都市オブジェクト |  | シート名：A.3.5_汎用都市オブジェクト |
| 7 | 拡張製品仕様書：汎用属性 |  | シート名：A.3.6_汎用属性 |
| 8 | 拡張製品仕様書：拡張品質要求 |  | シート名：A.3.7_拡張品質要求 |
| 9 | READMEテンプレート |  |  |
| 10 | 原典資料テンプレート |  |  |
| 11 | 索引図 |  | 3D都市モデルの整備範囲を示す図 |
| 12 | XMLSchema一式 | ― |  より取得できる。 |
| 13 | コードリスト一式 | ― |  より取得できる。 |

- 1及び2は、各都市で拡張製品仕様書を作成する際に必ず使用するファイルである。1は、標準製品仕様書に定義されるすべての地物型及びその定義が含まれている。
- 3から8は、標準製品仕様書にはない地物型や属性あるいは品質要求を拡張製品仕様書に追加する場合に使用するファイルである。
- 9から11は、拡張製品仕様書に従い3D都市モデルを整備し、成果品として納める際に使用するファイルである。
- 12及び13は、3D都市モデルの整備の際に必要となるファイルであり、成果品に含める必要があるファイルである。


