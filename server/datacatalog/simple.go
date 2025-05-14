package datacatalog

import (
	"context"
	"fmt"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/samber/lo"
)

type SimpleDatasetsResponse struct {
	Datasets []*SimpleDatasetsResponseDataset `json:"datasets"`
}

type SimpleDatasetsResponseDataset struct {
	ID               string   `json:"id"`
	Name             string   `json:"name"`
	Pref             string   `json:"pref"`
	PrefCode         string   `json:"pref_code"`
	City             *string  `json:"city"`
	CityCode         *string  `json:"city_code"`
	Ward             *string  `json:"ward"`
	WardCode         *string  `json:"ward_code"`
	Type             string   `json:"type"`
	TypeCode         string   `json:"type_en"`
	URL              string   `json:"url"`
	Layers           []string `json:"layers"`
	Year             int      `json:"year"`
	RegistrationYear int      `json:"registration_year"`
	Spec             string   `json:"spec"`
	Format           string   `json:"format"`
	LOD              *string  `json:"lod"`
	Texture          *bool    `json:"texture"`
}

func FetchSimplePlateauDatasets(ctx context.Context, r plateauapi.Repo) (*SimpleDatasetsResponse, error) {
	ds, err := r.Datasets(ctx, &plateauapi.DatasetsInput{
		IncludeTypes: []string{"plateau"},
	})
	if err != nil {
		return nil, err
	}

	res := &SimpleDatasetsResponse{}
	for _, dr := range ds {
		d, _ := dr.(*plateauapi.PlateauDataset)
		if d == nil {
			continue
		}

		prefID := d.GetPrefectureID()
		if prefID == nil {
			continue
		}

		var prefName, prefCode string
		{
			node, err := r.Node(ctx, *prefID)
			if err != nil {
				return nil, fmt.Errorf("failed to fetch prefecture: %w", err)
			}
			pref, _ := node.(*plateauapi.Prefecture)
			if pref != nil {
				prefName = pref.GetName()
				prefCode = pref.GetCode().String()
			}
		}

		var cityName, cityCode *string
		if p := d.GetCityID(); p != nil {
			node, err := r.Node(ctx, *p)
			if err != nil {
				return nil, fmt.Errorf("failed to fetch city: %w", err)
			}
			city, _ := node.(*plateauapi.City)
			if city != nil {
				cityName = lo.ToPtr(city.GetName())
				cityCode = lo.ToPtr(city.GetCode().String())
			}
		}

		var wardName, wardCode *string
		if p := d.GetWardID(); p != nil {
			node, err := r.Node(ctx, *p)
			if err != nil {
				return nil, fmt.Errorf("failed to fetch ward: %w", err)
			}
			ward, _ := node.(*plateauapi.Ward)
			if ward != nil {
				wardName = lo.ToPtr(ward.GetName())
				wardCode = lo.ToPtr(ward.GetCode().String())
			}
		}

		var typeName, typeCode string
		{
			node, err := r.Node(ctx, d.GetTypeID())
			if err != nil {
				return nil, fmt.Errorf("failed to fetch type: %w", err)
			}
			ty, _ := node.(plateauapi.DatasetType)
			if ty != nil {
				typeName = ty.GetName()
				typeCode = ty.GetCode()
			}
		}

		var spec string
		{
			node, err := r.Node(ctx, d.PlateauSpecMinorID)
			if err != nil {
				return nil, fmt.Errorf("failed to fetch spec: %w", err)
			}
			sp, _ := node.(*plateauapi.PlateauSpecMinor)
			if sp != nil {
				spec = sp.Version
			}
		}

		common := SimpleDatasetsResponseDataset{
			Name:             d.GetName(),
			Pref:             prefName,
			PrefCode:         prefCode,
			City:             cityName,
			CityCode:         cityCode,
			Ward:             wardName,
			WardCode:         wardCode,
			Type:             typeName,
			TypeCode:         typeCode,
			Year:             d.GetYear(),
			RegistrationYear: d.GetRegisterationYear(),
			Spec:             spec,
		}

		for _, di := range d.Items {
			f := simpleFormatName(di.GetFormat())
			if f == "" {
				continue
			}

			c := common
			c.ID = strings.TrimPrefix(string(di.GetID()), "di_")
			c.URL = di.GetURL()
			c.Layers = di.GetLayers()
			c.Format = f
			c.Texture = simpleTexture(di.Texture)
			if di.Lod != nil {
				c.LOD = lo.ToPtr(fmt.Sprintf("%d", *di.Lod))
			}

			res.Datasets = append(res.Datasets, &c)
		}
	}

	return res, nil
}

func simpleFormatName(f plateauapi.DatasetFormat) string {
	switch f {
	case plateauapi.DatasetFormatCesium3dtiles:
		return "3D Tiles"
	case plateauapi.DatasetFormatMvt:
		return "MVT"
	default:
		return ""
	}
}

func simpleTexture(f *plateauapi.Texture) *bool {
	if f == nil {
		return nil
	}
	switch *f {
	case plateauapi.TextureNone:
		return lo.ToPtr(false)
	case plateauapi.TextureTexture:
		return lo.ToPtr(true)
	default:
		return nil
	}
}
