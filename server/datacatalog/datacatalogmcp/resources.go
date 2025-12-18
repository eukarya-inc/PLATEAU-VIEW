package datacatalogmcp

import (
	"context"
	"fmt"
	"strings"

	"github.com/mark3labs/mcp-go/mcp"
	"github.com/mark3labs/mcp-go/server"
)

// RegisterResources registers datacatalogmcp resources
func RegisterResources(s *server.MCPServer) {
	resources, _ := HandleResourceList(context.Background())
	for _, resource := range resources {
		s.AddResource(resource, HandleResourceRead)
	}
}

// HandleResourceList returns a list of available datacatalogmcp resources
func HandleResourceList(_ context.Context) ([]mcp.Resource, error) {
	resources := []mcp.Resource{
		{
			URI:         "plateau://spatial-id",
			Name:        "空間ID（Spatial ID）仕様",
			Description: "空間IDの仕様と使い方。PLATEAUのCityGML検索で使用する3次元空間識別子の解説。",
			MIMEType:    "text/markdown",
		},
		{
			URI:         "plateau://glossary",
			Name:        "PLATEAU用語集・地物型一覧",
			Description: "PLATEAUで使用される用語と地物型（フィーチャータイプ）の定義一覧。",
			MIMEType:    "text/markdown",
		},
	}

	return resources, nil
}

// HandleResourceRead reads the content of a specific resource
func HandleResourceRead(_ context.Context, request mcp.ReadResourceRequest) ([]mcp.ResourceContents, error) {
	uri := request.Params.URI

	if !strings.HasPrefix(uri, "plateau://") {
		return nil, fmt.Errorf("invalid resource URI: %s", uri)
	}

	resourceName := strings.TrimPrefix(uri, "plateau://")

	switch resourceName {
	case "spatial-id":
		return []mcp.ResourceContents{
			mcp.TextResourceContents{
				URI:      uri,
				MIMEType: "text/markdown",
				Text:     spatialIDExplanation,
			},
		}, nil
	case "glossary":
		return []mcp.ResourceContents{
			mcp.TextResourceContents{
				URI:      uri,
				MIMEType: "text/markdown",
				Text:     glossaryContent,
			},
		}, nil
	default:
		return nil, fmt.Errorf("unknown resource: %s", uri)
	}
}

// glossaryContent contains PLATEAU terminology and feature type definitions
const glossaryContent = `# PLATEAU 用語集・地物型一覧

## 地物型（フィーチャータイプ）一覧

PLATEAUのCityGMLデータで使用される地物型の接頭辞と説明です。

| 接頭辞 | 地物型名 | 説明 |
|--------|----------|------|
| bldg | 建築物モデル | 建物や構造物の3D表現 |
| tran | 交通（道路）モデル | 道路の3D表現 |
| rwy | 交通（鉄道）モデル | 鉄道の3D表現 |
| trk | 交通（徒歩道）モデル | 歩道・遊歩道の3D表現 |
| squr | 交通（広場）モデル | 広場の3D表現 |
| wwy | 交通（航路）モデル | 航路の3D表現 |
| luse | 土地利用モデル | 土地利用区分の表現 |
| fld | 洪水浸水想定区域 | 河川氾濫による浸水リスク情報 |
| tnm | 津波浸水想定 | 津波による浸水リスク情報 |
| htd | 高潮浸水想定区域 | 高潮による浸水リスク情報 |
| ifld | 内水浸水想定区域 | 内水氾濫による浸水リスク情報 |
| rfld | ため池ハザードマップ | ため池決壊による浸水リスク情報 |
| lsld | 土砂災害警戒区域 | 土砂災害リスク情報 |
| urf | 都市計画決定情報モデル | 用途地域等の都市計画情報 |
| brid | 橋梁モデル | 橋の3D表現 |
| tun | トンネルモデル | トンネルの3D表現 |
| cons | その他の構造物モデル | 橋梁・トンネル以外の構造物 |
| frn | 都市設備モデル | 街灯・信号機等の都市設備 |
| unf | 地下埋設物モデル | 地下に埋設されたインフラ設備 |
| ubld | 地下街モデル | 地下街の3D表現 |
| veg | 植生モデル | 樹木・植栽の3D表現 |
| dem | 地形モデル | 地表面の標高データ |
| wtr | 水部モデル | 河川・湖沼等の水域 |
| area | 区域モデル | 各種区域の境界情報 |
| gen | 汎用都市オブジェクト | 標準で定義されていない汎用的な都市オブジェクト |
| app | アピアランスモデル | テクスチャ・マテリアル情報 |
| ext | 拡張地物 | 拡張製品仕様書で追加された地物 |

## 主要用語

### LOD（Level of Detail）
詳細度。3Dモデルの表現の精度を示す。LOD0からLOD4まであり、数字が大きいほど詳細。
- **LOD0**: 2D表現（フットプリント）
- **LOD1**: 箱型モデル（高さのみ）
- **LOD2**: 屋根形状を含む詳細モデル
- **LOD3**: 開口部（窓・ドア）を含む詳細モデル
- **LOD4**: 屋内構造を含む最詳細モデル

### CityGML
OGC（Open Geospatial Consortium）が策定した3D都市モデルのオープンデータ形式。XML/GMLベース。

### i-UR（i-Urban Revitalization）
日本独自のCityGML拡張仕様。都市計画情報等を表現するための追加スキーマを定義。

### メッシュコード
日本の標準地域メッシュを識別するコード。CityGMLファイルの分割単位として使用。
- **1次メッシュ**: 約80km四方
- **2次メッシュ**: 約10km四方
- **3次メッシュ**: 約1km四方（基準地域メッシュ）

### 空間ID（Spatial ID）
3次元空間を一意に識別するためのID。z/f/x/y形式で表現される。詳細は ` + "`plateau://spatial-id`" + ` を参照。

### PLATEAU仕様バージョン
国土交通省が定める3D都市モデルの標準仕様。年度ごとにバージョンが更新される（例：第3.0版、第4.0版）。

### 拡張製品仕様書
各都市が標準製品仕様書に基づいて作成する、都市固有の製品仕様書。標準にない地物や属性を追加可能。
`
