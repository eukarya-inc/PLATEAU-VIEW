package geocoding

import (
	"embed"
	"encoding/json"
	"sync"
)

//go:embed assets/areaCodes.json assets/areaRadii.json
var assetsFS embed.FS

type Area struct {
	Type   string  `json:"type"`
	Code   string  `json:"code"`
	Name   string  `json:"name"`
	Radius float64 `json:"radius"`
}

type Areas struct {
	Address string  `json:"address"`
	Areas   []*Area `json:"areas"`
}

type areaCodes struct {
	Prefectures    map[string]string `json:"prefectures"`
	Municipalities map[string]any    `json:"municipalities"`
}

type areaRadii map[string]float64

var (
	areaCodesData *areaCodes
	areaRadiiData areaRadii
	loadOnce      sync.Once
	loadErr       error
)

func loadAssets() error {
	loadOnce.Do(func() {
		// areaCodes.json
		data, err := assetsFS.ReadFile("assets/areaCodes.json")
		if err != nil {
			loadErr = err
			return
		}
		if err := json.Unmarshal(data, &areaCodesData); err != nil {
			loadErr = err
			return
		}

		// areaRadii.json
		data, err = assetsFS.ReadFile("assets/areaRadii.json")
		if err != nil {
			loadErr = err
			return
		}
		if err := json.Unmarshal(data, &areaRadiiData); err != nil {
			loadErr = err
			return
		}
	})
	return loadErr
}

func BuildAreas(code string, includeRadii bool) ([]*Area, error) {
	if err := loadAssets(); err != nil {
		return nil, err
	}

	if code == "" {
		return nil, nil
	}

	areas := make([]*Area, 0, 3)

	// Prefecture code (first 2 digits)
	prefCode := code[:2]
	prefName := areaCodesData.Prefectures[prefCode]

	// Municipality
	municipality := areaCodesData.Municipalities[code]
	if municipality == nil {
		return nil, nil
	}

	var municipalityName string
	var parentCode string

	switch m := municipality.(type) {
	case string:
		// Simple municipality: "千代田区"
		municipalityName = m
	case []any:
		// Complex municipality with parent: ["西区", "11100"] or ["さいたま市", ["11101", ...]]
		if len(m) >= 1 {
			if name, ok := m[0].(string); ok {
				municipalityName = name
			}
		}
		if len(m) >= 2 {
			if parent, ok := m[1].(string); ok {
				parentCode = parent
			}
		}
	}

	// Add municipality area
	areas = append(areas, &Area{
		Type:   "municipality",
		Code:   code,
		Name:   municipalityName,
		Radius: getRadius(code, includeRadii),
	})

	// Add parent municipality (e.g., city for ward)
	if parentCode != "" {
		parentMunicipality := areaCodesData.Municipalities[parentCode]
		if parentMunicipality != nil {
			var parentName string
			switch pm := parentMunicipality.(type) {
			case string:
				parentName = pm
			case []any:
				if len(pm) >= 1 {
					if name, ok := pm[0].(string); ok {
						parentName = name
					}
				}
			}
			if parentName != "" {
				areas = append(areas, &Area{
					Type:   "municipality",
					Code:   parentCode,
					Name:   parentName,
					Radius: getRadius(parentCode, includeRadii),
				})
			}
		}
	}

	// Add prefecture area
	areas = append(areas, &Area{
		Type:   "prefecture",
		Code:   prefCode,
		Name:   prefName,
		Radius: getRadius(prefCode, includeRadii),
	})

	return areas, nil
}

func getRadius(code string, includeRadii bool) float64 {
	if !includeRadii {
		return 0
	}
	if areaRadiiData == nil {
		return 0
	}
	return areaRadiiData[code]
}
