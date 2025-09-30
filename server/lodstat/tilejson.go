package lodstat

import (
	"encoding/json"
	"fmt"
)

const minZoom = 8

var tj = map[string]any{
	"tilejson":    "3.0.0",
	"name":        "plateau-lodstat",
	"description": "PLATEAU 3D都市モデルの地域メッシュ単位のLOD統計情報を提供するMVTタイルサービス",
	"scheme":      "xyz",
	"attribution": "<a href=\"https://www.mlit.go.jp/plateau/site-policy/\">国土交通省 PLATEAU</a>",
	"minzoom":     minZoom,
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

func tilesetJSON(host, ft string, level int) ([]byte, error) {
	res := map[string]any{}
	for k, v := range tj {
		res[k] = v
	}
	tj["tiles"] = []string{fmt.Sprintf("https://%s/lodstat/%s/%d/{z}/{x}/{y}.mvt", host, ft, level)}
	return json.Marshal(tj)
}
