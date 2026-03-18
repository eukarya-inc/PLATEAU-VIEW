package plateauapi

import (
	"encoding/json"
	"fmt"
)

// UnmarshalJSON implements custom JSON unmarshaling for Areas
func (a *Areas) UnmarshalJSON(data []byte) error {
	// Parse into raw map first
	var raw map[AreaType][]json.RawMessage
	if err := json.Unmarshal(data, &raw); err != nil {
		return err
	}

	*a = make(Areas)
	for areaType, rawAreas := range raw {
		areas := make([]Area, 0, len(rawAreas))
		for _, rawArea := range rawAreas {
			area, err := unmarshalArea(rawArea, areaType)
			if err != nil {
				return fmt.Errorf("failed to unmarshal area: %w", err)
			}
			areas = append(areas, area)
		}
		(*a)[areaType] = areas
	}

	return nil
}

func unmarshalArea(data json.RawMessage, areaType AreaType) (Area, error) {
	switch areaType {
	case AreaTypePrefecture:
		var p Prefecture
		if err := json.Unmarshal(data, &p); err != nil {
			return nil, err
		}
		return &p, nil
	case AreaTypeCity:
		var c City
		if err := json.Unmarshal(data, &c); err != nil {
			return nil, err
		}
		return &c, nil
	case AreaTypeWard:
		var w Ward
		if err := json.Unmarshal(data, &w); err != nil {
			return nil, err
		}
		return &w, nil
	case AreaTypeGlobal:
		var g GlobalArea
		if err := json.Unmarshal(data, &g); err != nil {
			return nil, err
		}
		return &g, nil
	default:
		return nil, fmt.Errorf("unknown area type: %s", areaType)
	}
}

// UnmarshalJSON implements custom JSON unmarshaling for Datasets
func (d *Datasets) UnmarshalJSON(data []byte) error {
	var raw map[DatasetTypeCategory][]json.RawMessage
	if err := json.Unmarshal(data, &raw); err != nil {
		return err
	}

	*d = make(Datasets)
	for cat, rawDatasets := range raw {
		datasets := make([]Dataset, 0, len(rawDatasets))
		for _, rawDs := range rawDatasets {
			ds, err := unmarshalDataset(rawDs, cat)
			if err != nil {
				return fmt.Errorf("failed to unmarshal dataset: %w", err)
			}
			datasets = append(datasets, ds)
		}
		(*d)[cat] = datasets
	}

	return nil
}

func unmarshalDataset(data json.RawMessage, cat DatasetTypeCategory) (Dataset, error) {
	switch cat {
	case DatasetTypeCategoryPlateau:
		var p PlateauDataset
		if err := json.Unmarshal(data, &p); err != nil {
			return nil, err
		}
		return &p, nil
	case DatasetTypeCategoryRelated:
		var r RelatedDataset
		if err := json.Unmarshal(data, &r); err != nil {
			return nil, err
		}
		return &r, nil
	case DatasetTypeCategoryGeneric:
		var g GenericDataset
		if err := json.Unmarshal(data, &g); err != nil {
			return nil, err
		}
		return &g, nil
	default:
		return nil, fmt.Errorf("unknown dataset category: %s", cat)
	}
}

// UnmarshalJSON implements custom JSON unmarshaling for DatasetTypes
func (d *DatasetTypes) UnmarshalJSON(data []byte) error {
	var raw map[DatasetTypeCategory][]json.RawMessage
	if err := json.Unmarshal(data, &raw); err != nil {
		return err
	}

	*d = make(DatasetTypes)
	for cat, rawTypes := range raw {
		types := make([]DatasetType, 0, len(rawTypes))
		for _, rawType := range rawTypes {
			dt, err := unmarshalDatasetType(rawType, cat)
			if err != nil {
				return fmt.Errorf("failed to unmarshal dataset type: %w", err)
			}
			types = append(types, dt)
		}
		(*d)[cat] = types
	}

	return nil
}

func unmarshalDatasetType(data json.RawMessage, cat DatasetTypeCategory) (DatasetType, error) {
	switch cat {
	case DatasetTypeCategoryPlateau:
		var p PlateauDatasetType
		if err := json.Unmarshal(data, &p); err != nil {
			return nil, err
		}
		return &p, nil
	case DatasetTypeCategoryRelated:
		var r RelatedDatasetType
		if err := json.Unmarshal(data, &r); err != nil {
			return nil, err
		}
		return &r, nil
	case DatasetTypeCategoryGeneric:
		var g GenericDatasetType
		if err := json.Unmarshal(data, &g); err != nil {
			return nil, err
		}
		return &g, nil
	default:
		return nil, fmt.Errorf("unknown dataset type category: %s", cat)
	}
}
