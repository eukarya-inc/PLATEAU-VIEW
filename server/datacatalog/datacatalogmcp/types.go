package datacatalogmcp

import (
	"context"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
)

// ReposHandler is an interface for accessing repos
// This avoids circular imports with datacatalog package
type ReposHandler interface {
	PrepareAndGetMergedRepo(ctx context.Context, project string, metadata plateaucms.MetadataList) plateauapi.Repo
}

// Common response metadata
type ResponseMetadata struct {
	TotalCount            int      `json:"total_count"`
	ReturnedCount         int      `json:"returned_count"`
	HasMore               bool     `json:"has_more"`
	RefinementSuggestions []string `json:"refinement_suggestions,omitempty"`
}

// Area types
type AreaInfo struct {
	ID           string  `json:"id"`
	Type         string  `json:"type"` // PREFECTURE, CITY, WARD, GLOBAL
	Code         string  `json:"code"`
	Name         string  `json:"name"`
	ParentID     *string `json:"parent_id,omitempty"`
	DatasetCount int     `json:"dataset_count"`
}

type SearchAreasInput struct {
	ParentCode     *string  `json:"parent_code,omitempty"`
	DatasetTypes   []string `json:"dataset_types,omitempty"`
	Categories     []string `json:"categories,omitempty"` // PLATEAU, RELATED, GENERIC
	AreaTypes      []string `json:"area_types,omitempty"` // PREFECTURE, CITY, WARD, GLOBAL
	SearchText     *string  `json:"search_text,omitempty"`
	IncludeParents *bool    `json:"include_parents,omitempty"`
	IncludeEmpty   *bool    `json:"include_empty,omitempty"`
	Deep           *bool    `json:"deep,omitempty"`
}

type SearchAreasResponse struct {
	Areas    []AreaInfo       `json:"areas"`
	Metadata ResponseMetadata `json:"metadata"`
}

type AreaParent struct {
	ID   string `json:"id"`
	Code string `json:"code"`
	Name string `json:"name"`
}

type AreaChild struct {
	ID   string `json:"id"`
	Code string `json:"code"`
	Name string `json:"name"`
}

type GetAreaResponse struct {
	ID                string      `json:"id"`
	Type              string      `json:"type"`
	Code              string      `json:"code"`
	Name              string      `json:"name"`
	Parent            *AreaParent `json:"parent,omitempty"`
	Children          []AreaChild `json:"children"`
	PlanarCrsEpsgCode *string     `json:"planar_crs_epsg_code,omitempty"` // City only
}

// Dataset types
type DatasetTypeInfo struct {
	Code     string `json:"code"`
	Name     string `json:"name"`
	Category string `json:"category"` // PLATEAU, RELATED, GENERIC
}

type DatasetAreaInfo struct {
	Prefecture *string `json:"prefecture,omitempty"`
	City       *string `json:"city,omitempty"`
	Ward       *string `json:"ward,omitempty"`
}

type DatasetInfo struct {
	ID               string          `json:"id"`
	Name             string          `json:"name"`
	Description      *string         `json:"description,omitempty"`
	Type             DatasetTypeInfo `json:"type"`
	Area             DatasetAreaInfo `json:"area"`
	Year             int             `json:"year"`
	RegistrationYear int             `json:"registration_year"`
	PlateauSpec      *string         `json:"plateau_spec,omitempty"`
	ItemCount        int             `json:"item_count"`
}

type SearchDatasetsInput struct {
	AreaCodes        []string `json:"area_codes,omitempty"`
	DatasetTypes     []string `json:"dataset_types,omitempty"`
	Categories       []string `json:"categories,omitempty"` // PLATEAU, RELATED, GENERIC
	PlateauSpec      *string  `json:"plateau_spec,omitempty"`
	Year             *int     `json:"year,omitempty"`
	RegistrationYear *int     `json:"registration_year,omitempty"`
	SearchText       *string  `json:"search_text,omitempty"`
	Shallow          *bool    `json:"shallow,omitempty"`
}

type SearchDatasetsResponse struct {
	Datasets []DatasetInfo    `json:"datasets"`
	Metadata ResponseMetadata `json:"metadata"`
}

type DatasetItemInfo struct {
	ID           string   `json:"id"`
	Name         string   `json:"name"`
	Format       string   `json:"format"`
	URL          string   `json:"url"`
	CompositeURL *string  `json:"composite_url,omitempty"`
	LatestURL    *string  `json:"latest_url,omitempty"`
	Lod          *int     `json:"lod,omitempty"`
	Interior     *bool    `json:"interior,omitempty"`
	Texture      *string  `json:"texture,omitempty"` // NONE, TEXTURE
	Layers       []string `json:"layers,omitempty"`
}

type DatasetAreaDetail struct {
	Prefecture *AreaParent `json:"prefecture,omitempty"`
	City       *AreaParent `json:"city,omitempty"`
	Ward       *AreaParent `json:"ward,omitempty"`
}

type PlateauSpecInfo struct {
	Name    string `json:"name"`
	Version string `json:"version"`
}

type GetDatasetResponse struct {
	ID               string            `json:"id"`
	Name             string            `json:"name"`
	Description      *string           `json:"description,omitempty"`
	Type             DatasetTypeInfo   `json:"type"`
	Area             DatasetAreaDetail `json:"area"`
	Year             int               `json:"year"`
	RegistrationYear int               `json:"registration_year"`
	PlateauSpec      *PlateauSpecInfo  `json:"plateau_spec,omitempty"`
	Groups           []string          `json:"groups,omitempty"`
	OpenDataURL      *string           `json:"open_data_url,omitempty"`
	Items            []DatasetItemInfo `json:"items"`
}

// Metadata types
type PlateauSpecMinorInfo struct {
	ID      string `json:"id"`
	Name    string `json:"name"`
	Version string `json:"version"`
}

type PlateauSpecMajorInfo struct {
	ID            string                 `json:"id"`
	MajorVersion  int                    `json:"major_version"`
	Year          int                    `json:"year"`
	MinorVersions []PlateauSpecMinorInfo `json:"minor_versions"`
}

type GetMetadataResponse struct {
	AvailableYears []int                  `json:"available_years"`
	PlateauSpecs   []PlateauSpecMajorInfo `json:"plateau_specs"`
	TotalAreas     int                    `json:"total_areas"`
	TotalDatasets  int                    `json:"total_datasets"`
}

// Dataset type list
type DatasetTypeListInfo struct {
	ID           string `json:"id"`
	Code         string `json:"code"`
	Name         string `json:"name"`
	Category     string `json:"category"`
	Year         *int   `json:"year,omitempty"`
	DatasetCount int    `json:"dataset_count"`
}

type ListDatasetTypesInput struct {
	Category    *string `json:"category,omitempty"` // PLATEAU, RELATED, GENERIC
	PlateauSpec *string `json:"plateau_spec,omitempty"`
	Year        *int    `json:"year,omitempty"`
}

type ListDatasetTypesResponse struct {
	DatasetTypes []DatasetTypeListInfo `json:"dataset_types"`
}

// CityGML types
type CityGMLAreaInfo struct {
	Prefecture string `json:"prefecture"`
	City       string `json:"city"`
}

type CityGMLDatasetInfo struct {
	ID               string          `json:"id"`
	Year             int             `json:"year"`
	RegistrationYear int             `json:"registration_year"`
	Area             CityGMLAreaInfo `json:"area"`
	URL              string          `json:"url"`
	GspatialURL      *string         `json:"gspatial_url,omitempty"`
	FeatureTypes     []string        `json:"feature_types"`
	PlateauSpec      string          `json:"plateau_spec"`
}

type SearchCityGMLInput struct {
	AreaCodes        []string `json:"area_codes,omitempty"`
	Year             *int     `json:"year,omitempty"`
	RegistrationYear *int     `json:"registration_year,omitempty"`
	PlateauSpec      *string  `json:"plateau_spec,omitempty"`
	FeatureTypes     []string `json:"feature_types,omitempty"`
}

type SearchCityGMLResponse struct {
	CityGMLDatasets []CityGMLDatasetInfo `json:"citygml_datasets"`
	Metadata        ResponseMetadata     `json:"metadata"`
}
