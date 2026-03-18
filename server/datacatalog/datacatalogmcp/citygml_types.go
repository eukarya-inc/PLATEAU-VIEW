package datacatalogmcp

// CityGML API related types

// GetCityGMLAttributesInput represents input for plateau_citygml_get_attributes tool
type GetCityGMLAttributesInput struct {
	URL          string   `json:"url"`
	BuildingIDs  []string `json:"building_ids"`
	SkipCodeList *bool    `json:"skip_code_list,omitempty"`
}

// GetCityGMLAttributesResponse represents response from plateau_citygml_get_attributes tool
type GetCityGMLAttributesResponse struct {
	Attributes []map[string]interface{} `json:"attributes"`
}

// GetCityGMLFeaturesInput represents input for plateau_citygml_get_features tool
type GetCityGMLFeaturesInput struct {
	URL        string   `json:"url"`
	SpatialIDs []string `json:"spatial_ids"`
}

// GetCityGMLFeaturesResponse represents response from plateau_citygml_get_features tool
type GetCityGMLFeaturesResponse struct {
	FeatureIDs []string                   `json:"feature_ids"`
	Reason     string                     `json:"reason,omitempty"`
	Hint       *GetCityGMLFeaturesHint    `json:"hint,omitempty"`
}

// GetCityGMLFeaturesHint provides guidance when results are empty
type GetCityGMLFeaturesHint struct {
	Message         string `json:"message,omitempty"`
	RecommendedZoom []int  `json:"recommended_zoom,omitempty"`
}

// GetGeoidHeightInput represents input for plateau_citygml_get_geoid_height tool
type GetGeoidHeightInput struct {
	Latitude  float64 `json:"latitude"`
	Longitude float64 `json:"longitude"`
}

// GetGeoidHeightResponse represents response from plateau_citygml_get_geoid_height tool
type GetGeoidHeightResponse struct {
	Latitude    float64 `json:"latitude"`
	Longitude   float64 `json:"longitude"`
	GeoidHeight float64 `json:"geoid_height"`
	Geoid       string  `json:"geoid"` // Formatted string for compatibility
}

// GetCityGMLFilesInput represents input for plateau_get_citygml_files tool
type GetCityGMLFilesInput struct {
	Condition    string   `json:"condition"`
	FeatureTypes []string `json:"feature_types,omitempty"`
}

// GetCityGMLFilesResponse represents response from plateau_get_citygml_files tool
type GetCityGMLFilesResponse struct {
	Cities       []CityGMLFilesCity            `json:"cities"`
	FeatureTypes map[string]CityGMLFeatureType `json:"featureTypes,omitempty"`
	Reason       string                        `json:"reason,omitempty"`
	Hint         *GetCityGMLFilesHint          `json:"hint,omitempty"`
}

// GetCityGMLFilesHint provides guidance when results are empty
type GetCityGMLFilesHint struct {
	Message string `json:"message,omitempty"`
}

// CityGMLFilesCity represents a city with CityGML files
type CityGMLFilesCity struct {
	CityCode         string                        `json:"cityCode"`
	CityName         string                        `json:"cityName"`
	Year             int                           `json:"year"`
	RegistrationYear int                           `json:"registrationYear"`
	Spec             string                        `json:"spec"`
	URL              string                        `json:"url"`
	Files            map[string][]CityGMLFile      `json:"files"`
	FeatureTypes     map[string]CityGMLFeatureType `json:"featureTypes,omitempty"`
}

// CityGMLFile represents a single CityGML file
type CityGMLFile struct {
	MeshCode string `json:"code"`
	MaxLOD   int    `json:"maxLod"`
	URL      string `json:"url"`
}

// CityGMLFeatureType represents a feature type
type CityGMLFeatureType struct {
	Name string `json:"name"`
}
