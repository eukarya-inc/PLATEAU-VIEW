package mvttilejson

import "strconv"

const formatMVT = "MVT"

// Input is the subset of dataset fields that Select consumes.
type Input struct {
	Name     string
	URL      string
	Format   string
	TypeCode string
	TypeName string
	Year     int
	LOD      *string
	Interior *bool // nil/false = non-interior; true = interior
	Layers   []string
	PrefCode string
	CityCode *string
	WardCode *string
	Pref     string
	City     string
	Ward     string
	Spec     string
}

// Select returns the single MVT dataset that matches the spec, or nil when
// no dataset matches. When Spec.YearMode is YearLatest, the dataset with the
// newest year wins.
func Select(datasets []Input, spec Spec) *Input {
	var best *Input
	for i := range datasets {
		d := &datasets[i]
		if d.Format != formatMVT {
			continue
		}
		if d.TypeCode != spec.Type {
			continue
		}
		if !cityMatches(spec.CityCode, d.CityCode, d.WardCode) {
			continue
		}
		if !lodMatches(spec.LOD, d.LOD) {
			continue
		}
		if spec.Interior != (d.Interior != nil && *d.Interior) {
			continue
		}
		if spec.YearMode == YearExact && d.Year != spec.Year {
			continue
		}

		if best == nil || d.Year > best.Year {
			best = d
		}
	}
	return best
}

func cityMatches(specCode string, cityCode, wardCode *string) bool {
	if wardCode != nil && *wardCode == specCode {
		return true
	}
	if cityCode != nil && *cityCode == specCode {
		return true
	}
	return false
}

func lodMatches(specLOD *int, dLOD *string) bool {
	if specLOD == nil {
		return dLOD == nil || *dLOD == ""
	}
	if dLOD == nil || *dLOD == "" {
		return false
	}
	n, err := strconv.Atoi(*dLOD)
	if err != nil {
		return false
	}
	return n == *specLOD
}
