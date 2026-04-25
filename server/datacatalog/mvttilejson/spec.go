package mvttilejson

import (
	"fmt"
	"strconv"
	"strings"
)

// YearMode controls how the year constraint is interpreted.
type YearMode int

const (
	// YearExact matches datasets whose year equals Spec.Year.
	YearExact YearMode = iota
	// YearLatest ignores Spec.Year and picks the dataset with the newest year.
	YearLatest
)

// Spec represents the parsed path parameter for an MVT TileJSON request.
//
// Format: <cityCode>-<type>[-lod<N>][-interior]-<year>
//   - cityCode: 5-digit municipal or ward code (pinpoint only)
//   - type:     dataset type code (e.g. "luse")
//   - lod<N>:   optional. When present, matches datasets whose LOD equals N.
//     When omitted, matches datasets without LOD only.
//   - interior: optional. When present, matches the CityGML 3.0 interior
//     variant; when omitted, only non-interior datasets match.
//   - year:     4-digit year, or the literal "latest"
//
// Examples:
//   - 13101-luse-2025
//   - 13101-luse-latest
//   - 13101-fld-lod1-2025
//   - 13101-bldg-lod3-interior-2025
type Spec struct {
	CityCode string
	Type     string
	LOD      *int // nil when LOD is omitted from the spec
	Interior bool
	Year     int // 0 when YearMode is YearLatest
	YearMode YearMode
}

// ParseSpec parses the path segment used by the MVT TileJSON endpoint.
func ParseSpec(s string) (Spec, error) {
	parts := strings.Split(s, "-")
	if len(parts) < 3 {
		return Spec{}, fmt.Errorf("invalid spec %q: expected <cityCode>-<type>[-lod<N>]-<year>", s)
	}

	cityCode := parts[0]
	if len(cityCode) != 5 {
		return Spec{}, fmt.Errorf("invalid spec %q: cityCode must be 5 digits", s)
	}
	if _, err := strconv.Atoi(cityCode); err != nil {
		return Spec{}, fmt.Errorf("invalid spec %q: cityCode must be numeric", s)
	}

	typeCode := parts[1]
	if typeCode == "" {
		return Spec{}, fmt.Errorf("invalid spec %q: empty type code", s)
	}

	var lod *int
	idx := 2
	if idx < len(parts)-1 && strings.HasPrefix(parts[idx], "lod") {
		n, err := strconv.Atoi(strings.TrimPrefix(parts[idx], "lod"))
		if err != nil || n < 0 {
			return Spec{}, fmt.Errorf("invalid spec %q: invalid lod value", s)
		}
		lod = &n
		idx++
	}
	interior := false
	if idx < len(parts)-1 && parts[idx] == "interior" {
		interior = true
		idx++
	}
	yearIdx := idx

	if len(parts) != yearIdx+1 {
		return Spec{}, fmt.Errorf("invalid spec %q: unexpected trailing segments", s)
	}

	yearStr := parts[yearIdx]
	yearMode := YearExact
	year := 0
	if yearStr == "latest" {
		yearMode = YearLatest
	} else {
		y, err := strconv.Atoi(yearStr)
		if err != nil || y < 1900 || y > 9999 {
			return Spec{}, fmt.Errorf("invalid spec %q: invalid year", s)
		}
		year = y
	}

	return Spec{
		CityCode: cityCode,
		Type:     typeCode,
		LOD:      lod,
		Interior: interior,
		Year:     year,
		YearMode: yearMode,
	}, nil
}
