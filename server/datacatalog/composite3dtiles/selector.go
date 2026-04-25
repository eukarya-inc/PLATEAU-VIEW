package composite3dtiles

import "strconv"

const format3DTiles = "3D Tiles"

// Input is the subset of dataset fields that Select consumes. The datacatalog
// package converts its SimpleDatasetsResponseDataset to this type to avoid an
// import cycle.
type Input struct {
	URL      string
	Format   string
	TypeCode string
	Year     int
	LOD      *string
	Interior *bool // nil/false = non-interior; true = interior (CityGML 3.0)
	Texture  *bool
	PrefCode string
	CityCode *string
	WardCode *string
}

type Candidate struct {
	URL      string
	AreaCode string // ward code preferred, otherwise city code
}

// Select filters datasets by spec and returns one URL per area.
//
// LOD rules:
//   - LODExact: keep only datasets whose LOD equals spec.LOD.
//   - LODMax:   keep datasets whose LOD is at most spec.LOD; per area, the
//     entry with the highest LOD wins. Texture rank breaks ties.
//
// Texture rules:
//   - TextureAuto: prefer textured; fall back to non-textured; fall back to
//     unspecified.
//   - TextureOnly: only datasets marked textured are kept.
//   - TextureNone: only datasets marked non-textured are kept.
func Select(datasets []Input, spec Spec) []Candidate {
	type entry struct {
		url     string
		year    int
		lod     int
		texture int // 2: textured, 1: non-textured, 0: unspecified
	}

	picked := map[string]entry{}

	for _, d := range datasets {
		if d.Format != format3DTiles {
			continue
		}
		if d.TypeCode != spec.Type {
			continue
		}
		if spec.YearMode == YearExact && d.Year != spec.Year {
			continue
		}
		if d.LOD == nil {
			continue
		}
		lod, err := strconv.Atoi(*d.LOD)
		if err != nil {
			continue
		}
		switch spec.LODMode {
		case LODExact:
			if lod != spec.LOD {
				continue
			}
		case LODMax:
			if lod > spec.LOD {
				continue
			}
		}

		isInterior := d.Interior != nil && *d.Interior
		switch spec.Interior {
		case InteriorExclude:
			if isInterior {
				continue
			}
		case InteriorOnly:
			if !isInterior {
				continue
			}
		}

		areaCode := ""
		if d.WardCode != nil && *d.WardCode != "" {
			areaCode = *d.WardCode
		} else if d.CityCode != nil && *d.CityCode != "" {
			areaCode = *d.CityCode
		}
		if areaCode == "" {
			continue
		}

		if !areaMatches(spec.Area, d.PrefCode, d.CityCode, d.WardCode) {
			continue
		}

		texRank := 0
		if d.Texture != nil {
			if *d.Texture {
				texRank = 2
			} else {
				texRank = 1
			}
		}

		switch spec.Texture {
		case TextureOnly:
			if texRank != 2 {
				continue
			}
		case TextureNone:
			if texRank != 1 {
				continue
			}
		}

		next := entry{url: d.URL, year: d.Year, lod: lod, texture: texRank}
		cur, ok := picked[areaCode]
		if !ok || better(next, cur) {
			picked[areaCode] = next
		}
	}

	out := make([]Candidate, 0, len(picked))
	for area, v := range picked {
		out = append(out, Candidate{URL: v.url, AreaCode: area})
	}
	return out
}

// better reports whether a should replace b. Year wins first (for YearLatest;
// no-op when years are equal as in YearExact), then LOD, then texture rank.
func better(a, b struct {
	url     string
	year    int
	lod     int
	texture int
}) bool {
	if a.year != b.year {
		return a.year > b.year
	}
	if a.lod != b.lod {
		return a.lod > b.lod
	}
	return a.texture > b.texture
}

func areaMatches(a Area, prefCode string, cityCode, wardCode *string) bool {
	switch a.Kind {
	case AreaAll:
		return true
	case AreaPref:
		return prefCode == a.Code
	case AreaCity:
		if wardCode != nil && *wardCode == a.Code {
			return true
		}
		if cityCode != nil && *cityCode == a.Code {
			return true
		}
		return false
	default:
		return false
	}
}
