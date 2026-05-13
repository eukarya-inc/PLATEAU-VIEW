package datacatalogv3

import (
	"fmt"
	"strings"
	"time"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/samber/lo"
)

const dicKeyAdmin = "admin"

func (i *PlateauFeatureItem) toWards(pref *plateauapi.Prefecture, city *plateauapi.City) (res []*plateauapi.Ward) {
	dic, _ := i.ReadDic()
	if dic == nil || len(dic[dicKeyAdmin]) == 0 {
		return nil
	}

	entries := dic[dicKeyAdmin]
	for _, entry := range entries {
		if entry.Code.String() == "" || entry.Description == "" {
			continue
		}

		_, name, _ := strings.Cut(entry.Description, " ")
		if name == "" {
			name = entry.Description
		}

		ward := &plateauapi.Ward{
			ID:             plateauapi.NewID(entry.Code.String(), plateauapi.TypeWard),
			Name:           name,
			Type:           plateauapi.AreaTypeWard,
			Code:           plateauapi.AreaCode(entry.Code.String()),
			PrefectureID:   pref.ID,
			PrefectureCode: pref.Code,
			CityID:         city.ID,
			CityCode:       city.Code,
			ParentID:       lo.ToPtr(city.ID),
		}

		res = append(res, ward)
	}

	return
}

type ToPlateauDatasetsOptions struct {
	ID          string
	CreatedAt   time.Time
	UpdatedAt   time.Time
	Area        *areaContext
	Spec        *plateauapi.PlateauSpecMinor
	DatasetType *plateauapi.PlateauDatasetType
	LayerNames  LayerNames
	FeatureType *FeatureType
	Year        int
	CMSInfo     CMSInfo
	IsFlow      bool // Flow model data (always beta, with [Flow] prefix)
}

func (i *PlateauFeatureItem) toDatasets(opts ToPlateauDatasetsOptions) (res []*plateauapi.PlateauDataset, warning []string) {
	if !opts.Area.IsValid() {
		warning = append(warning, fmt.Sprintf("plateau %s: invalid area", i.ID))
		return
	}

	if opts.DatasetType == nil {
		warning = append(warning, fmt.Sprintf("plateau %s: invalid dataset type", i.ID))
		return
	}

	if opts.FeatureType == nil {
		warning = append(warning, fmt.Sprintf("plateau %s: invalid feature type: %s", i.ID, opts.DatasetType.GetCode()))
		return
	}

	if opts.Spec == nil {
		warning = append(warning, fmt.Sprintf("plateau %s: invalid spec", i.ID))
		return
	}

	datasetSeeds, w := plateauDatasetSeedsFrom(i, opts)
	warning = append(warning, w...)
	for _, seed := range datasetSeeds {
		dataset, w := seedToDataset(seed)
		warning = append(warning, w...)
		if dataset != nil {
			res = append(res, dataset)
		}
	}

	return
}

const flowNamePrefix = "[Flowテスト用] "

func seedToDataset(seed plateauDatasetSeed) (res *plateauapi.PlateauDataset, warning []string) {
	if len(seed.AssetURLs) == 0 {
		// warning = append(warning, fmt.Sprintf("plateau %s %s: no asset urls", seed.TargetArea.GetCode(), seed.DatasetType.Code))
		return
	}

	sid := seed.GetID()
	id := plateauapi.NewID(sid, plateauapi.TypeDataset)

	seeds, w := plateauDatasetItemSeedFrom(seed)
	warning = append(warning, w...)
	items := lo.FilterMap(seeds, func(s plateauDatasetItemSeed, i int) (*plateauapi.PlateauDatasetItem, bool) {
		item := seedToDatasetItem(s, sid, seed.IsFlow)
		if item == nil {
			warning = append(warning, fmt.Sprintf("plateau %s %s[%d]: unknown dataset format: %s", seed.TargetArea.GetCode(), seed.DatasetType.Code, i, s.URL))
		}
		return item, item != nil
	})

	if len(items) == 0 {
		// warning is already reported by plateauDatasetItemSeedFrom
		warning = append(warning, fmt.Sprintf("plateau %s %s: no items", seed.TargetArea.GetCode(), seed.DatasetType.Code))
		return
	}

	// Check if any asset is interior
	isInterior := false
	for _, asset := range seed.Assets {
		if asset != nil && asset.Ex.Normal != nil && asset.Ex.Normal.Interior {
			isInterior = true
			break
		}
	}

	// Modify name and subname for interior datasets
	datasetName := seed.DatasetType.Name
	subname := seed.Subname
	subcode := seed.Subcode
	if isInterior {
		if subname != "" {
			subname = subname + "（屋内）"
		} else {
			datasetName = datasetName + "（屋内）"
		}
	}

	// Add [Flow] prefix for Flow datasets
	if seed.IsFlow {
		datasetName = flowNamePrefix + datasetName
	}

	res = &plateauapi.PlateauDataset{
		ID:                 id,
		Name:               standardItemName(datasetName, subname, seed.TargetArea.GetName()),
		Subname:            lo.EmptyableToPtr(subname),
		Subcode:            lo.EmptyableToPtr(subcode),
		Suborder:           seed.Suborder,
		Description:        lo.EmptyableToPtr(seed.Desc),
		Year:               seed.Area.CityItem.YearInt(),
		RegisterationYear:  seed.RegisterationYear,
		OpenDataURL:        lo.EmptyableToPtr(seed.OpenDataURL),
		PrefectureID:       seed.Area.PrefID,
		PrefectureCode:     seed.Area.PrefCode,
		CityID:             seed.Area.CityID,
		CityCode:           seed.Area.CityCode,
		WardID:             seed.WardID,
		WardCode:           seed.WardCode,
		TypeID:             seed.DatasetType.ID,
		TypeCode:           seed.DatasetType.Code,
		PlateauSpecMinorID: seed.Spec.ID,
		River:              seed.River,
		Admin:              seed.Admin,
		Groups:             seed.Groups,
		Items:              items,
		Ar:                 true,
	}

	return
}

func seedToDatasetItem(i plateauDatasetItemSeed, parentID string, isFlow bool) *plateauapi.PlateauDatasetItem {
	return &plateauapi.PlateauDatasetItem{
		ID:                  plateauapi.NewID(i.GetID(parentID), plateauapi.TypeDatasetItem),
		Name:                i.GetName(),
		URL:                 i.URL,
		FileSize:            int64PtrToIntPtr(i.FileSize),
		Layers:              i.Layers,
		Format:              i.Format,
		FormatVersion:       formatVersionFor(i.Format, isFlow),
		Lod:                 i.LOD,
		LodEx:               i.LODEx,
		Texture:             textureFrom(i.NoTexture),
		ParentID:            plateauapi.NewID(parentID, plateauapi.TypeDataset),
		FloodingScale:       i.FloodingScale,
		FloodingScaleSuffix: i.FloodingScaleSuffix,
	}
}

// int64PtrToIntPtr narrows a CMS asset size (kept as int64 internally so the
// arithmetic is unambiguous) into the *int that gqlgen emits for GraphQL Int.
// On 64-bit platforms — the only ones this server targets — the conversion is
// lossless for any plausible file size.
func int64PtrToIntPtr(v *int64) *int {
	if v == nil {
		return nil
	}
	n := int(*v)
	return &n
}

// formatVersionFor returns the format version string for a dataset item.
// Flow-converted 3D Tiles (FY2025+) are 1.1; legacy converters produce 1.0.
// Non–3D Tiles formats have no notion of format version.
func formatVersionFor(f plateauapi.DatasetFormat, isFlow bool) *string {
	if f != plateauapi.DatasetFormatCesium3dtiles {
		return nil
	}
	if isFlow {
		return lo.ToPtr("1.1")
	}
	return lo.ToPtr("1.0")
}
