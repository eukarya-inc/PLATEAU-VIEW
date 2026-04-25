package datacatalog

import (
	"context"
	"fmt"
	"sort"
	"strconv"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/samber/lo"
)

type SimpleDatasetsResponse struct {
	Datasets          []*SimpleDatasetsResponseDataset `json:"datasets"`
	CompositeTilesets []*SimpleCompositeTileset        `json:"composite_tilesets"`
	CityGML           []*SimpleCityGMLDataset          `json:"citygml"`
}

// SimpleCityGMLDataset describes a per-city CityGML merged.zip dataset
// derived from the G-Spatial Information Center dataset model.
type SimpleCityGMLDataset struct {
	ID               string   `json:"id"`
	Pref             string   `json:"pref"`
	PrefCode         string   `json:"pref_code"`
	City             string   `json:"city"`
	CityCode         string   `json:"city_code"`
	URL              string   `json:"url"`
	CompositeURL     *string  `json:"composite_url"`
	FeatureTypes     []string `json:"feature_types"`
	Year             int      `json:"year"`
	RegistrationYear int      `json:"registration_year"`
	Spec             string   `json:"spec"`
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
	CompositeURL     *string  `json:"composite_url"`
	Layers           []string `json:"layers"`
	Year             int      `json:"year"`
	RegistrationYear int      `json:"registration_year"`
	Spec             string   `json:"spec"`
	Format           string   `json:"format"`
	LOD              *string  `json:"lod"`
	Texture          *bool    `json:"texture"`
}

// SimpleCompositeTileset describes a virtual tileset.json that aggregates
// multiple per-area 3D Tiles datasets into a single URL.
type SimpleCompositeTileset struct {
	ID       string  `json:"id"`
	URL      string  `json:"url"`
	Area     string  `json:"area"` // "all" / "pref"
	PrefCode *string `json:"pref_code"`
	Pref     *string `json:"pref"`
	TypeCode string  `json:"type_en"`
	Type     string  `json:"type"`
	LOD      int     `json:"lod"`
	Texture  *bool   `json:"texture"` // nil=auto; true/false when explicitly filtered
	// Year is a 4-digit year string (e.g. "2025") for year-specific entries,
	// or "latest" for entries that resolve to the newest year per area.
	Year string `json:"year"`
}

func FetchSimplePlateauDatasets(ctx context.Context, r plateauapi.Repo, host string) (*SimpleDatasetsResponse, error) {
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

			if u := buildDatasetCompositeURL(host, &c); u != "" {
				c.CompositeURL = lo.ToPtr(u)
			}

			res.Datasets = append(res.Datasets, &c)
		}
	}

	res.CompositeTilesets = buildCompositeTilesets(host, res.Datasets)

	citygml, err := fetchSimpleCityGMLDatasets(ctx, r, host)
	if err != nil {
		return nil, err
	}
	res.CityGML = citygml

	return res, nil
}

func fetchSimpleCityGMLDatasets(ctx context.Context, r plateauapi.Repo, host string) ([]*SimpleCityGMLDataset, error) {
	ds, err := r.CitygmlDatasets(ctx, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch citygml datasets: %w", err)
	}

	res := make([]*SimpleCityGMLDataset, 0, len(ds))
	for _, d := range ds {
		if d == nil || d.URL == "" {
			continue
		}

		var prefName, prefCode string
		{
			node, err := r.Node(ctx, d.PrefectureID)
			if err != nil {
				return nil, fmt.Errorf("failed to fetch prefecture: %w", err)
			}
			if pref, _ := node.(*plateauapi.Prefecture); pref != nil {
				prefName = pref.GetName()
				prefCode = pref.GetCode().String()
			}
		}

		var cityName, cityCode string
		{
			node, err := r.Node(ctx, d.CityID)
			if err != nil {
				return nil, fmt.Errorf("failed to fetch city: %w", err)
			}
			if city, _ := node.(*plateauapi.City); city != nil {
				cityName = city.GetName()
				cityCode = city.GetCode().String()
			}
		}

		var spec string
		{
			node, err := r.Node(ctx, d.PlateauSpecMinorID)
			if err != nil {
				return nil, fmt.Errorf("failed to fetch spec: %w", err)
			}
			if sp, _ := node.(*plateauapi.PlateauSpecMinor); sp != nil {
				spec = sp.Version
			}
		}

		entry := &SimpleCityGMLDataset{
			ID:               strings.TrimPrefix(string(d.ID), "cg_"),
			Pref:             prefName,
			PrefCode:         prefCode,
			City:             cityName,
			CityCode:         cityCode,
			URL:              d.URL,
			FeatureTypes:     append([]string(nil), d.FeatureTypes...),
			Year:             d.Year,
			RegistrationYear: d.RegistrationYear,
			Spec:             spec,
		}
		if u := buildCityGMLCompositeURL(host, entry); u != "" {
			entry.CompositeURL = lo.ToPtr(u)
		}
		res = append(res, entry)
	}

	sort.Slice(res, func(i, j int) bool { return res[i].ID < res[j].ID })
	return res, nil
}

// buildDatasetCompositeURL returns the dynamic tileset URL that maps to a
// single dataset row (ward or city):
//   - 3D Tiles: composite tileset.json that defers to the underlying dataset
//   - MVT:      per-city TileJSON 3.0 wrapping the underlying dataset
//
// Empty when prerequisites are missing.
func buildDatasetCompositeURL(host string, d *SimpleDatasetsResponseDataset) string {
	if host == "" {
		return ""
	}
	if d.TypeCode == "" || d.Year == 0 {
		return ""
	}

	areaCode := ""
	if d.WardCode != nil && *d.WardCode != "" {
		areaCode = *d.WardCode
	} else if d.CityCode != nil && *d.CityCode != "" {
		areaCode = *d.CityCode
	}
	if areaCode == "" {
		return ""
	}

	switch d.Format {
	case "3D Tiles":
		if d.LOD == nil || *d.LOD == "" {
			return ""
		}
		return host + "/datacatalog/3dtiles/" + buildSpec(areaCode, d.TypeCode, *d.LOD, d.Texture, strconv.Itoa(d.Year)) + "/tileset.json"
	case "MVT":
		return host + "/datacatalog/mvt/" + buildMVTSpec(areaCode, d.TypeCode, d.LOD, strconv.Itoa(d.Year)) + "/tilejson.json"
	}
	return ""
}

// buildCityGMLCompositeURL returns a stable redirect URL that resolves to the
// per-city CityGML zip. Empty when prerequisites are missing.
func buildCityGMLCompositeURL(host string, d *SimpleCityGMLDataset) string {
	if host == "" || d == nil {
		return ""
	}
	if d.CityCode == "" || d.Year == 0 {
		return ""
	}
	return host + "/datacatalog/citygml/" + d.CityCode + "-" + strconv.Itoa(d.Year) + "/citygml.zip"
}

// buildMVTSpec assembles the path segment used by the MVT TileJSON endpoint.
// LOD is included only when the dataset specifies one; the year argument is
// either a 4-digit string or "latest".
func buildMVTSpec(area, typeCode string, lod *string, year string) string {
	parts := []string{area, typeCode}
	if lod != nil && *lod != "" {
		parts = append(parts, "lod"+*lod)
	}
	parts = append(parts, year)
	return strings.Join(parts, "-")
}

// buildSpec assembles the path segment used by the composite tileset endpoint.
// The lod argument is expected to be a numeric string ("1", "2", ...).
// The year argument is a 4-digit string ("2025") or the literal "latest".
func buildSpec(area, typeCode, lod string, texture *bool, year string) string {
	parts := []string{area, typeCode, "lod" + lod}
	if texture != nil {
		if *texture {
			parts = append(parts, "texture")
		} else {
			parts = append(parts, "notexture")
		}
	}
	parts = append(parts, year)
	return strings.Join(parts, "-")
}

// buildCompositeTilesets enumerates virtual tileset entries derived from the
// 3D Tiles datasets that exist. The "all" form covers Japan; "pref" is per
// prefecture. For each (area, type, lod) bucket, both year-specific entries
// and a "latest" entry (which resolves to the newest year per municipality)
// are emitted. Texture-specific variants are emitted only when both textured
// and non-textured data coexist within that bucket; otherwise the auto
// variant is the only useful URL.
func buildCompositeTilesets(host string, datasets []*SimpleDatasetsResponseDataset) []*SimpleCompositeTileset {
	if host == "" {
		return nil
	}

	type groupKey struct {
		area     string // "all" or "pref"
		prefCode string // empty when area == "all"
		prefName string
		typeCode string
		typeName string
		lod      int
		year     string // 4-digit year or "latest"
	}
	type groupAgg struct {
		hasTextured    bool
		hasNonTextured bool
	}

	groups := map[groupKey]*groupAgg{}

	add := func(k groupKey, texture *bool) {
		g, ok := groups[k]
		if !ok {
			g = &groupAgg{}
			groups[k] = g
		}
		if texture != nil {
			if *texture {
				g.hasTextured = true
			} else {
				g.hasNonTextured = true
			}
		}
	}

	for _, d := range datasets {
		if d == nil || d.Format != "3D Tiles" {
			continue
		}
		if d.LOD == nil || *d.LOD == "" {
			continue
		}
		lod, err := strconv.Atoi(*d.LOD)
		if err != nil {
			continue
		}
		if d.TypeCode == "" || d.Year == 0 {
			continue
		}

		years := []string{strconv.Itoa(d.Year), "latest"}
		for _, y := range years {
			// all-Japan
			add(groupKey{
				area:     "all",
				typeCode: d.TypeCode,
				typeName: d.Type,
				lod:      lod,
				year:     y,
			}, d.Texture)

			// per prefecture
			if d.PrefCode != "" {
				add(groupKey{
					area:     "pref",
					prefCode: d.PrefCode,
					prefName: d.Pref,
					typeCode: d.TypeCode,
					typeName: d.Type,
					lod:      lod,
					year:     y,
				}, d.Texture)
			}
		}
	}

	var out []*SimpleCompositeTileset
	for k, agg := range groups {
		areaCode := "all"
		if k.area == "pref" {
			areaCode = k.prefCode
		}
		base := func(textureSuffix *bool) *SimpleCompositeTileset {
			spec := buildSpec(areaCode, k.typeCode, strconv.Itoa(k.lod), textureSuffix, k.year)
			ent := &SimpleCompositeTileset{
				ID:       spec,
				URL:      host + "/datacatalog/3dtiles/" + spec + "/tileset.json",
				Area:     k.area,
				TypeCode: k.typeCode,
				Type:     k.typeName,
				LOD:      k.lod,
				Texture:  textureSuffix,
				Year:     k.year,
			}
			if k.area == "pref" {
				ent.PrefCode = lo.ToPtr(k.prefCode)
				ent.Pref = lo.ToPtr(k.prefName)
			}
			return ent
		}

		// auto (no texture suffix)
		out = append(out, base(nil))
		// texture-specific only when both kinds coexist
		if agg.hasTextured && agg.hasNonTextured {
			out = append(out, base(lo.ToPtr(true)))
			out = append(out, base(lo.ToPtr(false)))
		}
	}

	sort.Slice(out, func(i, j int) bool { return out[i].ID < out[j].ID })
	return out
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
