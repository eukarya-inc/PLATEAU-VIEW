package plateauapi

import (
	"strconv"
	"strings"
)

// buildPlateauItemDynamicURL returns the dynamic API URL that resolves to
// this PlateauDatasetItem (or the latest variant when latest is true).
// Returns empty when host is empty or required metadata is missing.
func buildPlateauItemDynamicURL(host string, item *PlateauDatasetItem, parent *PlateauDataset, latest bool) string {
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

	switch item.Format {
	case DatasetFormatCesium3dtiles:
		if item.Lod == nil {
			return ""
		}
		lod := strconv.Itoa(*item.Lod)
		spec := buildSpec3DTiles(areaCode, parent.TypeCode, lod, item.Texture, year)
		return host + "/datacatalog/3dtiles/" + spec + "/tileset.json"
	case DatasetFormatMvt:
		var lod *string
		if item.Lod != nil {
			s := strconv.Itoa(*item.Lod)
			lod = &s
		}
		spec := buildSpecMVT(areaCode, parent.TypeCode, lod, year)
		return host + "/datacatalog/mvt/" + spec + "/tilejson.json"
	}
	return ""
}

// buildSpec3DTiles assembles the path segment for the 3D Tiles composite endpoint.
func buildSpec3DTiles(area, typeCode, lod string, texture *Texture, year string) string {
	parts := []string{area, typeCode, "lod" + lod}
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
func buildSpecMVT(area, typeCode string, lod *string, year string) string {
	parts := []string{area, typeCode}
	if lod != nil && *lod != "" {
		parts = append(parts, "lod"+*lod)
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
