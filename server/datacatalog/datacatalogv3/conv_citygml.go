package datacatalogv3

import (
	"slices"

	"github.com/eukarya-inc/reearth-plateauview/server/datacatalog/plateauapi"
)

func toCityGMLs(all *AllData, regYear int) (map[plateauapi.ID]*plateauapi.CityGMLDataset, []string) {
	res := map[plateauapi.ID]*plateauapi.CityGMLDataset{}
	resCity := map[string]*plateauapi.CityGMLDataset{}
	dataMap := make(map[string]*GeospatialjpDataItem)
	cityMap := make(map[string]*CityItem)

	for _, d := range all.GeospatialjpDataItems {
		if d.CityGML == "" || d.MaxLOD == "" {
			continue
		}
		dataMap[d.City] = d
	}

	for _, city := range all.City {
		data := dataMap[city.ID]
		if data == nil {
			continue
		}

		prefCode := plateauapi.AreaCode(city.CityCode).PrefectureCode()
		d := &plateauapi.CityGMLDataset{
			ID:                 plateauapi.CityGMLDatasetIDFrom(plateauapi.AreaCode(city.CityCode)),
			URL:                data.CityGML,
			Year:               city.YearInt(),
			RegistrationYear:   regYear,
			PrefectureCode:     plateauapi.AreaCode(prefCode),
			PrefectureID:       plateauapi.NewID(prefCode, plateauapi.TypePrefecture),
			CityID:             plateauapi.NewID(city.CityCode, plateauapi.TypeCity),
			CityCode:           plateauapi.AreaCode(city.CityCode),
			FeatureTypes:       all.FeatureTypesOf(city.ID),
			PlateauSpecMinorID: plateauapi.PlateauSpecIDFrom(city.Spec),
			MetadataZipUrls:    city.MetadataZipURLs(),
			Admin: adminFrom(Admin{
				ItemID:      city.ID,
				Stage:       city.SDKStage(),
				CMSURL:      all.CMSInfo.ItemBaseURL(cityModel),
				MaxLODURLs:  []string{data.MaxLOD},
				CityGMLURLs: []string{data.CityGML},
			}),
		}

		cityMap[city.ID] = city
		res[d.ID] = d
		resCity[data.City] = d
	}

	// add citygml urls for sample data
	for ft, data := range all.Plateau {
		for _, d := range data {
			if !d.Sample || d.MaxLOD == "" || d.CityGML == "" /*|| !d.IsBeta()*/ {
				continue
			}

			citygml := resCity[d.City]
			if citygml == nil {
				continue
			}

			city := cityMap[d.City]
			if city == nil || !city.SDKPublic {
				continue
			}

			addCityGML(d.CityGML, d.MaxLOD, ft, citygml)
		}
	}

	for _, d := range all.Sample {
		if d.MaxLOD == "" || d.CityGML == "" {
			continue
		}

		citygml := resCity[d.City]
		if citygml == nil {
			continue
		}

		city := cityMap[d.City]
		if city == nil || !city.SDKPublic {
			continue
		}

		addCityGML(d.CityGML, d.MaxLOD, d.FeatureType, citygml)
	}

	return res, nil
}

func addCityGML(citygmlURL, maxlodURL, featureType string, citygml *plateauapi.CityGMLDataset) {
	if citygmlURL == "" || maxlodURL == "" {
		return
	}

	admin := plateauapi.AdminFrom(citygml.Admin)
	admin.CityGMLURLs = append(admin.CityGMLURLs, citygmlURL)
	admin.MaxLODURLs = append(admin.MaxLODURLs, maxlodURL)
	citygml.Admin = &admin

	if featureType != "" && !slices.Contains(citygml.FeatureTypes, featureType) {
		citygml.FeatureTypes = append(citygml.FeatureTypes, featureType)
	}
}
