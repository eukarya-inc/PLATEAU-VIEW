package composite3dtiles

import (
	"fmt"
	"strconv"
	"strings"
)

// AreaKind selects which datasets are composited.
type AreaKind int

const (
	// AreaAll composites datasets for all of Japan.
	AreaAll AreaKind = iota
	// AreaPref composites datasets within a prefecture (2-digit code).
	AreaPref
	// AreaCity composites datasets for a single city or ward (5-digit code).
	// Matches dataset.WardCode or dataset.CityCode.
	AreaCity
)

type Area struct {
	Kind AreaKind
	// Code is the 2-digit prefecture code or 5-digit municipal code.
	// Empty when Kind is AreaAll.
	Code string
}

// LODMode controls how the LOD constraint is interpreted.
type LODMode int

const (
	// LODExact matches datasets whose LOD equals Spec.LOD.
	LODExact LODMode = iota
	// LODMax matches datasets whose LOD is at most Spec.LOD, picking the
	// highest available LOD per area.
	LODMax
)

// TextureMode controls how the texture variant is selected.
type TextureMode int

const (
	// TextureAuto prefers textured but falls back to non-textured.
	TextureAuto TextureMode = iota
	// TextureOnly keeps only datasets explicitly marked as textured.
	TextureOnly
	// TextureNone keeps only datasets explicitly marked as non-textured.
	TextureNone
)

// Spec represents the parsed path parameter, e.g.:
//   - "all-bldg-lod2-2025"             (LOD = 2 exactly, texture auto)
//   - "all-bldg-maxlod2-2025"          (LOD ≤ 2, take max per area)
//   - "13-bldg-lod2-2025"              (Tokyo prefecture)
//   - "13101-bldg-lod2-2025"           (Chiyoda ward)
//   - "all-bldg-lod2-texture-2025"     (textured only)
//   - "all-bldg-lod2-notexture-2025"   (non-textured only)
type Spec struct {
	Area    Area
	Type    string // dataset type code (e.g. "bldg")
	LOD     int    // 1, 2, 3, ...
	LODMode LODMode
	Texture TextureMode
	Year    int // 4-digit year
}

func parseArea(s string) (Area, error) {
	if s == "all" {
		return Area{Kind: AreaAll}, nil
	}
	if len(s) != 2 && len(s) != 5 {
		return Area{}, fmt.Errorf("invalid area %q: must be \"all\", a 2-digit prefecture code, or a 5-digit municipal code", s)
	}
	if _, err := strconv.Atoi(s); err != nil {
		return Area{}, fmt.Errorf("invalid area %q: must be numeric", s)
	}
	if len(s) == 2 {
		return Area{Kind: AreaPref, Code: s}, nil
	}
	return Area{Kind: AreaCity, Code: s}, nil
}

// ParseSpec parses the path segment used by the composite tileset endpoint.
func ParseSpec(s string) (Spec, error) {
	parts := strings.Split(s, "-")
	if len(parts) < 4 {
		return Spec{}, fmt.Errorf("invalid spec %q: expected <area>-<type>-lod<N>[-notexture]-<year>", s)
	}

	area, err := parseArea(parts[0])
	if err != nil {
		return Spec{}, fmt.Errorf("invalid spec %q: %w", s, err)
	}

	typeCode := parts[1]
	if typeCode == "" {
		return Spec{}, fmt.Errorf("invalid spec %q: empty type code", s)
	}

	lodPart := parts[2]
	lodMode := LODExact
	lodNumStr := ""
	switch {
	case strings.HasPrefix(lodPart, "maxlod"):
		lodMode = LODMax
		lodNumStr = strings.TrimPrefix(lodPart, "maxlod")
	case strings.HasPrefix(lodPart, "lod"):
		lodNumStr = strings.TrimPrefix(lodPart, "lod")
	default:
		return Spec{}, fmt.Errorf("invalid spec %q: expected lod<N> or maxlod<N> in third segment", s)
	}
	lod, err := strconv.Atoi(lodNumStr)
	if err != nil || lod < 0 {
		return Spec{}, fmt.Errorf("invalid spec %q: invalid lod value", s)
	}

	texture := TextureAuto
	yearIdx := 3
	if len(parts) >= 5 {
		switch parts[3] {
		case "texture":
			texture = TextureOnly
			yearIdx = 4
		case "notexture":
			texture = TextureNone
			yearIdx = 4
		}
	}

	if len(parts) != yearIdx+1 {
		return Spec{}, fmt.Errorf("invalid spec %q: unexpected trailing segments", s)
	}

	year, err := strconv.Atoi(parts[yearIdx])
	if err != nil || year < 1900 || year > 9999 {
		return Spec{}, fmt.Errorf("invalid spec %q: invalid year", s)
	}

	return Spec{
		Area:    area,
		Type:    typeCode,
		LOD:     lod,
		LODMode: lodMode,
		Texture: texture,
		Year:    year,
	}, nil
}
