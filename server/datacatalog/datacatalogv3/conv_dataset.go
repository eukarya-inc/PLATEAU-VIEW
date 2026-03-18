package datacatalogv3

import (
	"fmt"
	"net/url"
	"path"
	"strings"
	"time"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/samber/lo"
)

func riverAdminFrom(admin string) *plateauapi.RiverAdmin {
	switch admin {
	case "国":
		fallthrough
	case "natl":
		return lo.ToPtr(plateauapi.RiverAdminNational)
	case "都道府県":
		fallthrough
	case "pref":
		return lo.ToPtr(plateauapi.RiverAdminPrefecture)
	}
	return nil
}

func toRiverAdminName(a plateauapi.RiverAdmin) string {
	switch a {
	case plateauapi.RiverAdminNational:
		return "国"
	case plateauapi.RiverAdminPrefecture:
		return "都道府県"
	}
	return ""
}

func textureFrom(notexture *bool) *plateauapi.Texture {
	if notexture == nil {
		return nil
	}
	if *notexture {
		return lo.ToPtr(plateauapi.TextureNone)
	}
	return lo.ToPtr(plateauapi.TextureTexture)
}

func datasetFormatFromOrDetect(f string, url string) plateauapi.DatasetFormat {
	if f != "" {
		return datasetFormatFrom(f)
	}
	return detectDatasetFormatFromURL(url)
}

func datasetFormatFrom(f string) plateauapi.DatasetFormat {
	switch strings.ToLower(f) {
	case "geojson":
		return plateauapi.DatasetFormatGeojson
	case "3dtiles":
		fallthrough
	case "3d tiles":
		return plateauapi.DatasetFormatCesium3dtiles
	case "czml":
		return plateauapi.DatasetFormatCzml
	case "gtfs":
		fallthrough
	case "gtfs-realtime":
		return plateauapi.DatasetFormatGtfsRealtime
	case "gltf":
		return plateauapi.DatasetFormatGltf
	case "mvt":
		return plateauapi.DatasetFormatMvt
	case "tiles":
		return plateauapi.DatasetFormatTiles
	case "tms":
		return plateauapi.DatasetFormatTms
	case "wms":
		return plateauapi.DatasetFormatWms
	case "csv":
		return plateauapi.DatasetFormatCSV
	}
	return ""
}

func detectDatasetFormatFromURL(url string) plateauapi.DatasetFormat {
	name := strings.ToLower(nameFromURL(url))

	switch {
	case strings.HasSuffix(name, ".geojson"):
		return plateauapi.DatasetFormatGeojson
	case strings.HasSuffix(name, ".czml"):
		return plateauapi.DatasetFormatCzml
	case strings.HasSuffix(name, "{z}/{x}/{y}.pbf"):
		fallthrough
	case strings.HasSuffix(name, ".mvt"):
		return plateauapi.DatasetFormatMvt
	case name == "tileset.json":
		return plateauapi.DatasetFormatCesium3dtiles
	case strings.HasSuffix(name, ".csv"):
		return plateauapi.DatasetFormatCSV
	case strings.HasSuffix(name, ".gltf"):
		return plateauapi.DatasetFormatGltf
	case strings.HasSuffix(name, "{z}/{x}/{y}.png"):
		return plateauapi.DatasetFormatTiles
	}

	return ""
}

func standardItemID(name string, areaCode plateauapi.AreaCode, ex string) string {
	if ex != "" {
		ex = fmt.Sprintf("_%s", ex)
	}
	return fmt.Sprintf("%s_%s%s", areaCode, name, ex)
}

func standardItemName(name, subname, areaName string) string {
	var suffix string
	if areaName != "" {
		suffix = fmt.Sprintf("（%s）", areaName)
		name = strings.TrimSuffix(name, suffix)
	}

	space := ""
	if subname != "" {
		space = " "
	}

	return fmt.Sprintf("%s%s%s%s", name, space, subname, suffix)
}

func layerNamesFrom(layer string) []string {
	if layer == "" {
		return nil
	}

	return lo.Map(strings.Split(layer, ","), func(s string, _ int) string {
		return strings.TrimSpace(s)
	})
}

type Admin struct {
	ItemID    string
	CMSURL    string
	Stage     stage
	CreatedAt time.Time
	UpdatedAt time.Time
	// common
	SubAreaCode    string
	CityGMLAssetID string
	CityGMLURLs    []string
	MaxLODURLs     []string
}

func adminFrom(admin Admin) any {
	stage := ""
	if admin.Stage != stageGA {
		if admin.Stage == "" {
			stage = string(stageAlpha)
		} else {
			stage = string(admin.Stage)
		}
	}

	cmsurl := ""
	if admin.CMSURL != "" && admin.ItemID != "" {
		cmsurl = admin.CMSURL + admin.ItemID
	}

	res := &plateauapi.Admin{
		CMSItemID:      admin.ItemID,
		CMSURL:         cmsurl,
		Stage:          stage,
		SubAreaCode:    admin.SubAreaCode,
		CityGMLAssetID: admin.CityGMLAssetID,
		CityGMLURLs:    admin.CityGMLURLs,
		MaxLODURLs:     admin.MaxLODURLs,
		CreatedAt:      lo.EmptyableToPtr(admin.CreatedAt),
		UpdatedAt:      lo.EmptyableToPtr(admin.UpdatedAt),
	}

	if !admin.CreatedAt.IsZero() {
		res.CreatedAt = &admin.CreatedAt
	}

	if !admin.UpdatedAt.IsZero() {
		res.UpdatedAt = &admin.UpdatedAt
	}

	if res.IsEmpty() {
		return nil
	}

	return res
}

func assetURLFromFormat(u string, f plateauapi.DatasetFormat) string {
	if u == "" {
		return ""
	}

	u2, err := url.Parse(u)
	if err != nil {
		return u
	}

	dir := path.Dir(u2.Path)
	ext := path.Ext(u2.Path)
	base := path.Base(u2.Path)
	name := strings.TrimSuffix(base, ext)
	isArchive := ext == ".zip" || ext == ".7z"

	u2.Path = assetRootPath(u2.Path)
	if f == plateauapi.DatasetFormatCesium3dtiles {
		if !isArchive {
			// not CMS asset
			return u
		}

		u2.Path = path.Join(u2.Path, "tileset.json")
		return u2.String()
	} else if f == plateauapi.DatasetFormatTiles || f == plateauapi.DatasetFormatMvt {
		us := ""
		if !isArchive {
			// not CMS asset
			us = u
		} else {
			ext := ""
			if f == plateauapi.DatasetFormatMvt {
				ext = "mvt"
			} else {
				ext = "png"
			}

			u2.Path = path.Join(u2.Path, "{z}/{x}/{y}."+ext)
			us = u2.String()
		}

		return strings.ReplaceAll(strings.ReplaceAll(us, "%7B", "{"), "%7D", "}")
	} else if f == plateauapi.DatasetFormatTms {
		if !isArchive {
			// not CMS asset
			return u
		}
		return u2.String()
	} else if (f == plateauapi.DatasetFormatCzml) && isArchive {
		u2.Path = path.Join(dir, name, fmt.Sprintf("%s.%s", name, strings.ToLower(string(f))))
		return u2.String()
	}
	return u
}

func assetRootPath(p string) string {
	fn := strings.TrimSuffix(path.Base(p), path.Ext(p))
	return path.Join(path.Dir(p), fn)
}
