package plateauapi

import (
	"strconv"
	"strings"
)

// isInteriorDataset detects the CityGML 3.0 interior split by checking the
// parent dataset's ID for the "_interior" suffix added by datacatalogv3.
func isInteriorDataset(d *PlateauDataset) bool {
	return d != nil && strings.Contains(string(d.ID), "_interior")
}

// BuildPlateauItemDynamicURL returns the dynamic API URL that resolves to
// this PlateauDatasetItem (or the latest variant when latest is true).
// Returns empty when host is empty or required metadata is missing.
func BuildPlateauItemDynamicURL(host string, item *PlateauDatasetItem, parent *PlateauDataset, latest bool) string {
	if host == "" || item == nil || parent == nil {
		return ""
	}
	if parent.TypeCode == "" || parent.Year == 0 {
		return ""
	}

	areaCode := ""
	if parent.WardCode != nil && string(*parent.WardCode) != "" {
		areaCode = string(*parent.WardCode)
	} else if parent.CityCode != nil && string(*parent.CityCode) != "" {
		areaCode = string(*parent.CityCode)
	}
	if areaCode == "" {
		return ""
	}

	year := "latest"
	if !latest {
		year = strconv.Itoa(parent.Year)
	}

	interior := isInteriorDataset(parent)

	switch item.Format {
	case DatasetFormatCesium3dtiles:
		if item.Lod == nil {
			return ""
		}
		lod := strconv.Itoa(*item.Lod)
		spec := buildSpec3DTiles(areaCode, parent.TypeCode, lod, interior, item.Texture, year)
		return host + "/datacatalog/3dtiles/" + spec + "/tileset.json"
	case DatasetFormatMvt:
		var lod *string
		if item.Lod != nil {
			s := strconv.Itoa(*item.Lod)
			lod = &s
		}
		spec := buildSpecMVT(areaCode, parent.TypeCode, lod, interior, year)
		return host + "/datacatalog/mvt/" + spec + "/tilejson.json"
	}
	return ""
}

// buildSpec3DTiles assembles the path segment for the 3D Tiles composite endpoint.
func buildSpec3DTiles(area, typeCode, lod string, interior bool, texture *Texture, year string) string {
	parts := []string{area, typeCode, "lod" + lod}
	if interior {
		parts = append(parts, "interior")
	}
	if texture != nil {
		switch *texture {
		case TextureTexture:
			parts = append(parts, "texture")
		case TextureNone:
			parts = append(parts, "notexture")
		}
	}
	parts = append(parts, year)
	return strings.Join(parts, "-")
}

// buildSpecMVT assembles the path segment for the MVT TileJSON endpoint.
func buildSpecMVT(area, typeCode string, lod *string, interior bool, year string) string {
	parts := []string{area, typeCode}
	if lod != nil && *lod != "" {
		parts = append(parts, "lod"+*lod)
	}
	if interior {
		parts = append(parts, "interior")
	}
	parts = append(parts, year)
	return strings.Join(parts, "-")
}

// buildCityGMLDynamicURL returns the citygml redirect URL for the given dataset
// (or the latest variant when latest is true).
func buildCityGMLDynamicURL(host string, d *CityGMLDataset, latest bool) string {
	if host == "" || d == nil {
		return ""
	}
	cityCode := string(d.CityCode)
	if cityCode == "" {
		return ""
	}
	year := "latest"
	if !latest {
		if d.Year == 0 {
			return ""
		}
		year = strconv.Itoa(d.Year)
	}
	return host + "/datacatalog/citygml/" + cityCode + "-" + year + "/citygml.zip"
}
