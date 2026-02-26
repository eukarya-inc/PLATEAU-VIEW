package preparegspatialjp

const udx = "udx"
const indexmap = "_indexmap_op.pdf"

var citygmlDic = map[string]string{
	"codelists":     "コードリスト",
	"metadata":      "メタデータ",
	"schemas":       "CityGMLスキーマ",
	"specification": "3D都市モデルのための拡張製品仕様書",
	"indexmap":      "索引図（PDF）",
	"natl":          "国管理",
	"pref":          "都道府県管理",
}

func resolveFeatureTypeName(code string, featureTypeNames map[string]string) string {
	if name, ok := featureTypeNames[code]; ok {
		return name
	}
	return code
}

var relatedDataTypes = []string{
	"shelter",
	"landmark",
	"station",
	"park",
	"railway",
	"emergency_route",
	"border",
}

var relatedDataTypeMap = map[string]string{
	"shelter":         "避難施設情報",
	"landmark":        "ランドマーク情報",
	"station":         "鉄道駅情報",
	"park":            "公園情報",
	"railway":         "鉄道情報",
	"emergency_route": "緊急輸送道路情報",
	"border":          "行政界情報",
}
