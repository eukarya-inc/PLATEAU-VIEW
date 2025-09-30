package lodstat

import (
	"encoding/json"
	"fmt"
	"maps"
)

const minZoomLevel3 = 8
const minZoomLevel2 = 3 // Minimum zoom for level 2 and auto mode

var tj = map[string]any{
	"tilejson":    "3.0.0",
	"name":        "plateau-lodstat",
	"description": "PLATEAU 3D都市モデルの地域メッシュ単位のLOD統計情報を提供するMVTタイルサービス",
	"scheme":      "xyz",
	"attribution": "<a href=\"https://www.mlit.go.jp/plateau/site-policy/\">国土交通省 PLATEAU</a>",
	"minzoom":     minZoomLevel3,
	"vector_layers": []map[string]any{
		{
			"id":          "lodstat",
			"description": "PLATEAU 3D都市モデルの地域メッシュ単位のLOD統計情報レイヤー",
			"fields": map[string]string{
				"featureType": "string",
				"level":       "number",
				"meshCode":    "string",
				"maxLod":      "number",
				"features":    "number",
				"fileSize":    "number",
				"lod0":        "boolean",
				"lod1":        "boolean",
				"lod2":        "boolean",
				"lod3":        "boolean",
				"lod4":        "boolean",
				"lod0Count":   "number",
				"lod1Count":   "number",
				"lod2Count":   "number",
				"lod3Count":   "number",
				"lod4Count":   "number",
			},
		},
	},
}

func tilesetJSON(host, ft string, levelStr string, level int) ([]byte, error) {
	res := map[string]any{}
	maps.Copy(res, tj)

	// Set minzoom based on level
	if level == 2 {
		res["minzoom"] = minZoomLevel2
	} else {
		res["minzoom"] = minZoomLevel3
	}

	res["tiles"] = []string{fmt.Sprintf("https://%s/lodstat/mvt/%s/%s/{z}/{x}/{y}.mvt", host, ft, levelStr)}
	return json.Marshal(res)
}
