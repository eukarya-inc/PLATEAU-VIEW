package datacatalog

import (
	"context"
	"fmt"
	"net/url"
	"path"
	"strconv"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/reearth/reearthx/util"
	"github.com/samber/lo"
)

type CityGMLFilesCity struct {
	CityCode         string                        `json:"cityCode"`
	CityName         string                        `json:"cityName"`
	Year             int                           `json:"year"`
	RegistrationYear int                           `json:"registrationYear"`
	Spec             string                        `json:"spec"`
	URL              string                        `json:"url"`
	Files            CityGMLFiles                  `json:"files"`
	MetadataZipUrls  []string                      `json:"metadataZipUrls"`
	FeatureTypes     map[string]CityGMLFeatureType `json:"featureTypes,omitempty"`
}

type CityGMLFiles = map[string][]CityGMLFile

type CityGMLFile struct {
	MeshCode string `json:"code"`
	MaxLOD   int    `json:"maxLod"`
	URL      string `json:"url"`
	FileSize *int64 `json:"fileSize,omitempty"`
	Features *int   `json:"features,omitempty"`
	LOD0     *int   `json:"lod0,omitempty"`
	LOD1     *int   `json:"lod1,omitempty"`
	LOD2     *int   `json:"lod2,omitempty"`
	LOD3     *int   `json:"lod3,omitempty"`
	LOD4     *int   `json:"lod4,omitempty"`
}

type CityGMLFeatureType struct {
	Name string `json:"name"`
}

func FetchCityGMLFiles(ctx context.Context, r plateauapi.Repo, id string, datasetTypes []plateauapi.DatasetType) (*CityGMLFilesCity, error) {
	n, err := r.Node(ctx, plateauapi.CityGMLDatasetIDFrom(plateauapi.AreaCode(id)))
	if err != nil {
		return nil, err
	}

	citygml, ok := n.(*plateauapi.CityGMLDataset)
	if !ok || citygml == nil || citygml.URL == "" || citygml.PlateauSpecMinorID == "" {
		return nil, nil
	}

	n, err = r.Node(ctx, citygml.PlateauSpecMinorID)
	if err != nil {
		return nil, err
	}

	spec, ok := n.(*plateauapi.PlateauSpecMinor)
	if !ok || spec == nil {
		return nil, nil
	}

	n, err = r.Node(ctx, citygml.CityID)
	if err != nil {
		return nil, err
	}

	city, ok := n.(*plateauapi.City)
	if !ok || city == nil {
		return nil, nil
	}

	admin := plateauapi.AdminFrom(citygml.Admin)

	var gurls []*url.URL
	citygmlAssetID := admin.CityGMLAssetID
	if citygmlAssetID != "" {
		mds := plateaucms.GetAllCMSMetadataFromContext(ctx)
		md := mds.FindByYear(citygml.RegistrationYear)
		if md == nil {
			return nil, fmt.Errorf("failed to find cms")
		}

		cms, err := md.CMS()
		if err != nil || cms == nil {
			return nil, fmt.Errorf("failed to init cms: %w", err)
		}

		asset, err := cms.Asset(ctx, citygmlAssetID)
		if err != nil {
			return nil, fmt.Errorf("failed to get asset: %w", err)
		}

		assetBase, err := url.Parse(asset.URL)
		if err != nil {
			return nil, fmt.Errorf("failed to parse asset url: %w", err)
		}

		assetBase.Path = path.Dir(assetBase.Path)
		gurls = gmlURLs(asset.File.Paths(), assetBase)
	}

	data, err := fetchCSVs(ctx, admin.MaxLODURLs, admin.CityGMLURLs)
	if err != nil {
		return nil, err
	}

	files := csvToCityGMLFilesResponse(data, gurls)

	// Build feature types map from provided datasetTypes
	featureTypes := make(map[string]CityGMLFeatureType)
	if datasetTypes != nil {
		for k := range files {
			for _, t := range datasetTypes {
				if t.GetCode() == k {
					featureTypes[k] = CityGMLFeatureType{Name: t.GetName()}
					break
				}
			}
		}
	}

	return &CityGMLFilesCity{
		CityCode:         string(citygml.CityCode),
		CityName:         city.Name,
		Year:             citygml.Year,
		RegistrationYear: citygml.RegistrationYear,
		Spec:             spec.Version,
		URL:              citygml.URL,
		Files:            files,
		MetadataZipUrls:  citygml.MetadataZipUrls,
		FeatureTypes:     featureTypes,
	}, nil
}

func citygmlItemURLFrom(base, p, typeCode string) string {
	b := path.Base(base)
	base = strings.TrimSuffix(base, b)
	u, _ := url.JoinPath(base, nameWithoutExt(b), "udx", typeCode, p)
	return u
}

func gmlURLs(paths []string, base *url.URL) []*url.URL {
	res := lo.FilterMap(paths, func(u string, _ int) (*url.URL, bool) {
		if path.Ext(u) != ".gml" {
			return nil, false
		}

		u2, err := url.Parse(u)
		if err != nil {
			return nil, false
		}

		if base == nil {
			return u2, true
		}

		fu := util.CloneRef(base)
		fu.Path = path.Join(fu.Path, u)
		return fu, true
	})

	return res
}

func isNumeric(r rune) bool {
	return r >= '0' && r <= '9'
}

func nameWithoutExt(name string) string {
	return strings.TrimSuffix(name, path.Ext(name))
}

func parseLOD(s string) *int {
	if s == "" {
		return nil
	}
	if v, err := strconv.ParseInt(s, 10, 64); err == nil {
		v := int(v)
		return &v
	}
	if v, err := strconv.ParseBool(s); err == nil {
		x := 0
		if v {
			x = 1
		}
		return &x
	}
	return nil
}
