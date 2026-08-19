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
	LatestDatasets    []*SimpleLatestDataset           `json:"latest_datasets"`
	CompositeTilesets []*SimpleCompositeTileset        `json:"composite_tilesets"`
	CityGML           []*SimpleCityGMLDataset          `json:"citygml"`
	LatestCityGML     []*SimpleLatestCityGMLDataset    `json:"latest_citygml"`
}

// SimpleLatestDataset is a per-city 3D Tiles or MVT entry whose URL resolves
// to the dataset of the newest available year for that (city, type, lod)
// bucket. The URL changes content when a new maintenance year is published,
// without requiring clients to update the URL itself.
type SimpleLatestDataset struct {
	ID       string  `json:"id"`
	Name     string  `json:"name"`
	Pref     string  `json:"pref"`
	PrefCode string  `json:"pref_code"`
	City     *string `json:"city"`
	CityCode *string `json:"city_code"`
	Ward     *string `json:"ward"`
	WardCode *string `json:"ward_code"`
	Type     string  `json:"type"`
	TypeCode string  `json:"type_en"`
	URL      string  `json:"url"`
	// FileSize is the CMS-reported size of the asset at URL, in bytes.
	// Mirrors the underlying year-specific dataset chosen as the latest.
	FileSize *int64   `json:"file_size,omitempty"`
	Layers   []string `json:"layers"`
	Year     string   `json:"year"` // always "latest"
	Format   string   `json:"format"`
	// FormatVersion is the 3D Tiles format version ("1.0" or "1.1") for
	// 3D Tiles rows; nil for MVT and other formats. See SimpleDatasetsResponseDataset.FormatVersion.
	FormatVersion *string `json:"format_version,omitempty"`
	LOD           *string `json:"lod"`
	Texture       *bool   `json:"texture"`
	Interior      *bool   `json:"interior,omitempty"`
}

// SimpleLatestCityGMLDataset is a per-city CityGML entry whose URL redirects
// to the zip of the newest available year for that city.
type SimpleLatestCityGMLDataset struct {
	ID       string `json:"id"`
	Pref     string `json:"pref"`
	PrefCode string `json:"pref_code"`
	City     string `json:"city"`
	CityCode string `json:"city_code"`
	URL      string `json:"url"`
	// FileSize mirrors the underlying year-specific CityGML zip size in bytes.
	FileSize     *int64   `json:"file_size,omitempty"`
	FeatureTypes []string `json:"feature_types"`
	Year         string   `json:"year"` // always "latest"
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
	FileSize         *int64   `json:"file_size,omitempty"`
	CompositeURL     *string  `json:"composite_url"`
	FeatureTypes     []string `json:"feature_types"`
	Year             int      `json:"year"`
	RegistrationYear int      `json:"registration_year"`
	Spec             string   `json:"spec"`
}

type SimpleDatasetsResponseDataset struct {
	ID       string  `json:"id"`
	Name     string  `json:"name"`
	Pref     string  `json:"pref"`
	PrefCode string  `json:"pref_code"`
	City     *string `json:"city"`
	CityCode *string `json:"city_code"`
	Ward     *string `json:"ward"`
	WardCode *string `json:"ward_code"`
	Type     string  `json:"type"`
	TypeCode string  `json:"type_en"`
	URL      string  `json:"url"`
	// FileSize is the CMS-reported size of the asset at URL, in bytes.
	FileSize         *int64   `json:"file_size,omitempty"`
	CompositeURL     *string  `json:"composite_url"`
	Layers           []string `json:"layers"`
	Year             int      `json:"year"`
	RegistrationYear int      `json:"registration_year"`
	Spec             string   `json:"spec"`
	Format           string   `json:"format"`
	// FormatVersion is the 3D Tiles format version ("1.0" or "1.1") for
	// 3D Tiles rows; nil for MVT. Flow-converted 3D Tiles (FY2025+) are 1.1.
	FormatVersion *string `json:"format_version,omitempty"`
	LOD           *string `json:"lod"`
	Texture       *bool   `json:"texture"`
	// Interior は CityGML 3.0 の屋内モデル区分を表す。`true` の行は
	// interior 専用データセットを指し、`false` は明示的な非 interior、
	// `null` は interior 区分がそもそも存在しないデータ（LOD1 や MVT 等）。
	Interior *bool `json:"interior,omitempty"`
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
	Interior *bool   `json:"interior,omitempty"`
	Texture  *bool   `json:"texture"` // nil=auto; true/false when explicitly filtered
	// Year is a 4-digit year string (e.g. "2025") for year-specific entries,
	// or "latest" for entries that resolve to the newest year per area.
	Year string `json:"year"`
	// FormatVersion is the wrapper Asset.version that the composite tileset.json
	// will report — the maximum version among the underlying children, or "1.0"
	// when none of them is known to be 1.1.
	FormatVersion string `json:"format_version"`
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

		// Interior datasets (CityGML 3.0 屋内モデル) are split as dedicated
		// PlateauDataset entities whose IDs carry an "_interior" segment.
		// We surface that flag only when set — non-interior rows leave the
		// field nil because legacy data has no interior concept at all.
		var interiorFlag *bool
		if strings.Contains(string(d.GetID()), "_interior") {
			interiorFlag = lo.ToPtr(true)
		}

		common := SimpleDatasetsResponseDataset{
			Name:             d.GetName(),
			Pref:             prefName,
			PrefCode:         prefCode,
			City:             cityName,
			CityCode:         cityCode,
			Ward:             wardName,
			WardCode:         wardCode,
			Interior:         interiorFlag,
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
			c.FileSize = intPtrToInt64Ptr(di.GetFileSize())
			c.Layers = di.GetLayers()
			c.Format = f
			c.FormatVersion = di.FormatVersion
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

	sort.Slice(res.Datasets, func(i, j int) bool { return res.Datasets[i].ID < res.Datasets[j].ID })

	res.CompositeTilesets = buildCompositeTilesets(host, res.Datasets)
	res.LatestDatasets = buildLatestDatasets(host, res.Datasets)

	citygml, err := fetchSimpleCityGMLDatasets(ctx, r, host)
	if err != nil {
		return nil, err
	}
	res.CityGML = citygml
	res.LatestCityGML = buildLatestCityGMLDatasets(host, citygml)

	return res, nil
}

// buildLatestDatasets enumerates per-city `-latest` 3D Tiles/MVT entries by
// selecting, within each (areaCode, typeCode, lod, format, texture) bucket,
// the row with the newest year and rewriting its URL to the dynamic
// `-latest` endpoint.
func buildLatestDatasets(host string, datasets []*SimpleDatasetsResponseDataset) []*SimpleLatestDataset {
	if host == "" {
		return nil
	}

	type bucketKey struct {
		areaCode string
		typeCode string
		lod      string // "" when LOD is absent
		format   string
		interior bool
		// Texture variants are kept separate so non-textured rows do not
		// shadow textured rows of the same year.
		texture string // "true"/"false"/""
	}

	type entry struct {
		row *SimpleDatasetsResponseDataset
	}

	picked := map[bucketKey]*entry{}

	for _, d := range datasets {
		if d == nil {
			continue
		}
		areaCode := ""
		if d.WardCode != nil && *d.WardCode != "" {
			areaCode = *d.WardCode
		} else if d.CityCode != nil && *d.CityCode != "" {
			areaCode = *d.CityCode
		}
		if areaCode == "" || d.TypeCode == "" || d.Year == 0 {
			continue
		}
		if d.Format != "3D Tiles" && d.Format != "MVT" {
			continue
		}
		lod := ""
		if d.LOD != nil {
			lod = *d.LOD
		}
		tex := ""
		if d.Texture != nil {
			if *d.Texture {
				tex = "true"
			} else {
				tex = "false"
			}
		}
		k := bucketKey{areaCode: areaCode, typeCode: d.TypeCode, lod: lod, format: d.Format, interior: isTrue(d.Interior), texture: tex}
		if e := picked[k]; e == nil || d.Year > e.row.Year {
			picked[k] = &entry{row: d}
		}
	}

	out := make([]*SimpleLatestDataset, 0, len(picked))
	for _, e := range picked {
		d := e.row
		areaCode := ""
		if d.WardCode != nil && *d.WardCode != "" {
			areaCode = *d.WardCode
		} else if d.CityCode != nil && *d.CityCode != "" {
			areaCode = *d.CityCode
		}
		var url string
		switch d.Format {
		case "3D Tiles":
			if d.LOD == nil || *d.LOD == "" {
				continue
			}
			url = host + "/datacatalog/3dtiles/" + buildSpec(areaCode, d.TypeCode, *d.LOD, isTrue(d.Interior), d.Texture, "latest") + "/tileset.json"
		case "MVT":
			url = host + "/datacatalog/mvt/" + buildMVTSpec(areaCode, d.TypeCode, d.LOD, isTrue(d.Interior), "latest") + "/tilejson.json"
		default:
			continue
		}
		out = append(out, &SimpleLatestDataset{
			ID:            d.ID,
			Name:          d.Name,
			Pref:          d.Pref,
			PrefCode:      d.PrefCode,
			City:          d.City,
			CityCode:      d.CityCode,
			Ward:          d.Ward,
			WardCode:      d.WardCode,
			Type:          d.Type,
			TypeCode:      d.TypeCode,
			URL:           url,
			FileSize:      d.FileSize,
			Layers:        d.Layers,
			Year:          "latest",
			Format:        d.Format,
			FormatVersion: d.FormatVersion,
			LOD:           d.LOD,
			Texture:       d.Texture,
			Interior:      d.Interior,
		})
	}
	sort.Slice(out, func(i, j int) bool { return out[i].URL < out[j].URL })
	return out
}

// buildLatestCityGMLDatasets enumerates per-city `-latest` CityGML redirect
// entries: one per cityCode, using the newest-year row.
func buildLatestCityGMLDatasets(host string, datasets []*SimpleCityGMLDataset) []*SimpleLatestCityGMLDataset {
	if host == "" {
		return nil
	}

	picked := map[string]*SimpleCityGMLDataset{}
	for _, d := range datasets {
		if d == nil || d.CityCode == "" || d.Year == 0 {
			continue
		}
		if cur, ok := picked[d.CityCode]; !ok || d.Year > cur.Year {
			picked[d.CityCode] = d
		}
	}

	out := make([]*SimpleLatestCityGMLDataset, 0, len(picked))
	for _, d := range picked {
		out = append(out, &SimpleLatestCityGMLDataset{
			ID:           d.ID,
			Pref:         d.Pref,
			PrefCode:     d.PrefCode,
			City:         d.City,
			CityCode:     d.CityCode,
			URL:          host + "/datacatalog/citygml/" + d.CityCode + "-latest/citygml.zip",
			FileSize:     d.FileSize,
			FeatureTypes: append([]string(nil), d.FeatureTypes...),
			Year:         "latest",
		})
	}
	sort.Slice(out, func(i, j int) bool { return out[i].CityCode < out[j].CityCode })
	return out
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
			FileSize:         intPtrToInt64Ptr(d.FileSize),
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
		return host + "/datacatalog/3dtiles/" + buildSpec(areaCode, d.TypeCode, *d.LOD, isTrue(d.Interior), d.Texture, strconv.Itoa(d.Year)) + "/tileset.json"
	case "MVT":
		return host + "/datacatalog/mvt/" + buildMVTSpec(areaCode, d.TypeCode, d.LOD, isTrue(d.Interior), strconv.Itoa(d.Year)) + "/tilejson.json"
	}
	return ""
}

// isTrue reports whether b is non-nil and points to true.
func isTrue(b *bool) bool { return b != nil && *b }

// intPtrToInt64Ptr widens the *int that gqlgen emits for GraphQL Int into the
// *int64 the simple API exposes, so downstream JSON consumers see a stable
// numeric type even on 32-bit builds.
func intPtrToInt64Ptr(v *int) *int64 {
	if v == nil {
		return nil
	}
	n := int64(*v)
	return &n
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
func buildMVTSpec(area, typeCode string, lod *string, interior bool, year string) string {
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

// buildSpec assembles the path segment used by the composite tileset endpoint.
// The lod argument is expected to be a numeric string ("1", "2", ...).
// The year argument is a 4-digit string ("2025") or the literal "latest".
func buildSpec(area, typeCode, lod string, interior bool, texture *bool, year string) string {
	parts := []string{area, typeCode, "lod" + lod}
	if interior {
		parts = append(parts, "interior")
	}
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
		interior bool
		year     string // 4-digit year or "latest"
	}
	type groupAgg struct {
		hasTextured    bool
		hasNonTextured bool
		// Wrapper version per texture-variant: the auto variant sees every
		// child, while -texture / -notexture filter to one texture rank, so
		// each may resolve to a different max format version.
		maxVersionAny         string
		maxVersionTextured    string
		maxVersionNonTextured string
	}

	groups := map[groupKey]*groupAgg{}

	bumpVersion := func(cur, v string) string {
		if v == "" {
			return cur
		}
		if v > cur {
			return v
		}
		return cur
	}

	add := func(k groupKey, texture *bool, version string) {
		g, ok := groups[k]
		if !ok {
			g = &groupAgg{}
			groups[k] = g
		}
		if texture != nil {
			if *texture {
				g.hasTextured = true
				g.maxVersionTextured = bumpVersion(g.maxVersionTextured, version)
			} else {
				g.hasNonTextured = true
				g.maxVersionNonTextured = bumpVersion(g.maxVersionNonTextured, version)
			}
		}
		g.maxVersionAny = bumpVersion(g.maxVersionAny, version)
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

		interior := isTrue(d.Interior)
		version := ""
		if d.FormatVersion != nil {
			version = *d.FormatVersion
		}
		years := []string{strconv.Itoa(d.Year), "latest"}
		for _, y := range years {
			// all-Japan
			add(groupKey{
				area:     "all",
				typeCode: d.TypeCode,
				typeName: d.Type,
				lod:      lod,
				interior: interior,
				year:     y,
			}, d.Texture, version)

			// per prefecture
			if d.PrefCode != "" {
				add(groupKey{
					area:     "pref",
					prefCode: d.PrefCode,
					prefName: d.Pref,
					typeCode: d.TypeCode,
					typeName: d.Type,
					lod:      lod,
					interior: interior,
					year:     y,
				}, d.Texture, version)
			}
		}
	}

	var out []*SimpleCompositeTileset
	for k, agg := range groups {
		areaCode := "all"
		if k.area == "pref" {
			areaCode = k.prefCode
		}
		pickVersion := func(textureSuffix *bool) string {
			var v string
			switch {
			case textureSuffix == nil:
				v = agg.maxVersionAny
			case *textureSuffix:
				v = agg.maxVersionTextured
			default:
				v = agg.maxVersionNonTextured
			}
			if v == "" {
				// Default to 1.0 to match the wrapper Build() emits when no
				// child reports a known version.
				return "1.0"
			}
			return v
		}
		base := func(textureSuffix *bool) *SimpleCompositeTileset {
			spec := buildSpec(areaCode, k.typeCode, strconv.Itoa(k.lod), k.interior, textureSuffix, k.year)
			interior := k.interior
			ent := &SimpleCompositeTileset{
				ID:            spec,
				URL:           host + "/datacatalog/3dtiles/" + spec + "/tileset.json",
				Area:          k.area,
				TypeCode:      k.typeCode,
				Type:          k.typeName,
				LOD:           k.lod,
				Interior:      lo.If(interior, lo.ToPtr(true)).Else(nil),
				Texture:       textureSuffix,
				Year:          k.year,
				FormatVersion: pickVersion(textureSuffix),
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
