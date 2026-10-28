package cmsintsetup

import (
	"context"
	"encoding/csv"
	"errors"
	"fmt"
	"io"
	"strings"
	"unicode/utf8"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/eukarya-inc/jpareacode"
	"github.com/k0kubun/pp/v3"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/log"
	"golang.org/x/exp/slices"
)

type SetupCityItemsInput struct {
	ProjectID string    `json:"projectId"`
	DataURL   string    `json:"dataUrl"`
	DataBody  io.Reader `json:"-"`
	// Deprecated: cities that already exist are always skipped or resumed, so this has no effect.
	Force  bool `json:"force"`
	Offset int  `json:"offset"`
	Limit  int  `json:"limit"`
	DryRun bool `json:"dryrun"`
}

type SetupCSVItem struct {
	Prefecture string   `json:"pref"`
	Name       string   `json:"city"`
	NameEn     string   `json:"city_en"`
	Code       string   `json:"city_code"`
	Features   []string `json:"features"`
	// Fixes lists the normalizations applied to the row, such as a restored leading zero of the city code.
	Fixes []string `json:"fixes,omitempty"`
}

type SetupAction string

const (
	// SetupActionCreate creates the city item and all of its child items.
	SetupActionCreate SetupAction = "create"
	// SetupActionResume completes a city item that exists but whose child items are partly missing,
	// which happens when a previous run stopped in the middle of a city.
	SetupActionResume SetupAction = "resume"
	// SetupActionSkip does nothing because the city item and all of its child items exist.
	SetupActionSkip SetupAction = "skip"
)

type SetupCityItemsResult struct {
	Created []string `json:"created"`
	Resumed []string `json:"resumed"`
	Skipped []string `json:"skipped"`
	// Fixes lists the normalizations applied to the CSV, prefixed with the city name.
	Fixes []string `json:"fixes"`
}

func (r *SetupCityItemsResult) String() string {
	var b strings.Builder
	fmt.Fprintf(&b, "create: %d, resume: %d, skip: %d, fixes: %d", len(r.Created), len(r.Resumed), len(r.Skipped), len(r.Fixes))
	if len(r.Resumed) > 0 {
		fmt.Fprintf(&b, "\nresume: %s", strings.Join(r.Resumed, ", "))
	}
	for _, f := range r.Fixes {
		fmt.Fprintf(&b, "\nfix: %s", f)
	}
	return b.String()
}

func SetupCityItems(ctx context.Context, s *Services, inp SetupCityItemsInput, onprogress func(i, l int, c SetupCSVItem, action SetupAction)) (*SetupCityItemsResult, error) {
	if inp.ProjectID == "" {
		return nil, fmt.Errorf("modelId is required")
	}

	if inp.DataURL == "" && inp.DataBody == nil {
		return nil, fmt.Errorf("dataUrl is required")
	}

	log.Infofc(ctx, "cmsintegrationv3: setup city items to %s", inp.ProjectID)

	// get model info
	modelIDs := map[string]string{}
	models, err := s.CMS.GetModels(ctx, inp.ProjectID)
	if err != nil {
		return nil, fmt.Errorf("failed to get models: %w", err)
	}

	for _, m := range models.Models {
		if !strings.HasPrefix(m.Key, cmsintegrationcommon.ModelPrefix) {
			continue
		}
		code := strings.TrimPrefix(m.Key, cmsintegrationcommon.ModelPrefix)
		modelIDs[code] = m.ID
	}
	if len(modelIDs) == 0 || modelIDs[cmsintegrationcommon.CityModel] == "" || modelIDs[cmsintegrationcommon.RelatedModel] == "" {
		return nil, fmt.Errorf("no models found")
	}

	log.Infofc(ctx, "cmsintegrationv3: models\n%s", pp.Sprint(modelIDs))

	cityModel := modelIDs[cmsintegrationcommon.CityModel]

	// get feature types
	featureTypes, err := s.PCMS.PlateauFeatureTypes(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get feature types: %w", err)
	}

	featureTypeCodes := featureTypes.Codes()
	datasetTypes, err := s.PCMS.DatasetTypes(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get dataset types: %w", err)
	}

	relatedDataTypes := datasetTypes.Codes(plateaucms.DatasetCategoryRelated)

	// parse data
	setupItems, features, err := getAndParseSetupCSV(ctx, s, inp.DataURL, inp.DataBody)
	if err != nil {
		return nil, fmt.Errorf("failed to get and parse data: %w", err)
	}

	if inp.Offset > 0 {
		setupItems = setupItems[min(inp.Offset, len(setupItems)):]
	}

	if inp.Limit > 0 {
		setupItems = setupItems[:min(inp.Limit, len(setupItems))]
	}

	for _, f := range features {
		if modelIDs[f] == "" {
			return nil, fmt.Errorf("model id for %s is not found", f)
		}
	}

	// existing city items are indexed by city code to make the setup safe to run again
	existingCities, err := listModelItems(ctx, s.CMS, cityModel)
	if err != nil {
		return nil, fmt.Errorf("failed to get city items: %w", err)
	}

	existingCityByCode := map[string]*cms.Item{}
	for _, c := range existingCities {
		code := cmsintegrationcommon.CityItemFrom(c, featureTypeCodes).CityCode
		if code == "" {
			// test items without a city code are not managed by this setup
			continue
		}
		existingCityByCode[code] = c
	}

	log.Infofc(ctx, "cmsintegrationv3: setup features: %v", features)
	log.Infofc(ctx, "cmsintegrationv3: setup %d items (%d city items already exist)", len(setupItems), len(existingCityByCode))

	children := &cityChildren{
		cms:              s.CMS,
		modelIDs:         modelIDs,
		features:         features,
		relatedDataTypes: relatedDataTypes,
		itemsByModel:     map[string][]*cms.Item{},
	}

	res := &SetupCityItemsResult{}

	// process cities
	for i, item := range setupItems {
		for _, f := range item.Fixes {
			res.Fixes = append(res.Fixes, item.Name+": "+f)
		}

		cityItem := existingCityByCode[item.Code]
		var missing []string
		action := SetupActionCreate
		if cityItem != nil {
			missing = children.missingKeys(cityItem)
			if len(missing) == 0 {
				action = SetupActionSkip
			} else {
				action = SetupActionResume
			}
		}

		switch action {
		case SetupActionCreate:
			res.Created = append(res.Created, item.Name)
		case SetupActionResume:
			res.Resumed = append(res.Resumed, item.Name)
		case SetupActionSkip:
			res.Skipped = append(res.Skipped, item.Name)
		}

		if onprogress != nil {
			onprogress(i, len(setupItems), item, action)
		}

		if inp.DryRun || action == SetupActionSkip {
			continue
		}

		if cityItem == nil {
			cityCMSItem := (&cmsintegrationcommon.CityItem{
				Prefecture: item.Prefecture,
				CityName:   item.Name,
				CityNameEn: item.NameEn,
				CityCode:   item.Code,
			}).CMSItem(featureTypeCodes)

			cityItem, err = s.CMS.CreateItem(ctx, cityModel, cityCMSItem.Fields, cityCMSItem.MetadataFields)
			if err != nil {
				return res, fmt.Errorf("failed to create city item (%d/%d): %w", i, len(setupItems), err)
			}
			missing = children.keys()
		}

		refs, err := children.ensure(ctx, cityItem, item, missing)
		if err != nil {
			return res, fmt.Errorf("failed to create child items of %s (%d/%d): %w", item.Name, i, len(setupItems), err)
		}

		featureItemIDs := map[string]string{}
		for _, f := range features {
			featureItemIDs[f] = refs[f]
		}

		if _, err := s.CMS.UpdateItem(ctx, cityItem.ID, (&cmsintegrationcommon.CityItem{
			References:        featureItemIDs,
			RelatedDataset:    refs[cmsintegrationcommon.RelatedModel],
			GeospatialjpIndex: refs[cmsintegrationcommon.GeospatialjpIndex],
			GeospatialjpData:  refs[cmsintegrationcommon.GeospatialjpData],
		}).CMSItem(featureTypeCodes).Fields, nil); err != nil {
			return res, fmt.Errorf("failed to update city item (%d/%d): %w", i, len(setupItems), err)
		}
	}

	log.Infofc(ctx, "cmsintegrationv3: setup city items done: %s", res)

	return res, nil
}

// cityChildren creates the items that hang off a city item: the related dataset, geospatialjp-index,
// geospatialjp-data and one item per feature type.
type cityChildren struct {
	cms              cms.Interface
	modelIDs         map[string]string
	features         []string
	relatedDataTypes []string
	// itemsByModel caches all items of a model, loaded only when a city has to be resumed
	itemsByModel map[string][]*cms.Item
}

func (c *cityChildren) keys() []string {
	return append([]string{
		cmsintegrationcommon.RelatedModel,
		cmsintegrationcommon.GeospatialjpIndex,
		cmsintegrationcommon.GeospatialjpData,
	}, c.features...)
}

// missingKeys returns the keys of the child items that the city item does not reference yet.
func (c *cityChildren) missingKeys(city *cms.Item) []string {
	var res []string
	for _, k := range c.keys() {
		if cityRef(city, k) == "" {
			res = append(res, k)
		}
	}
	return res
}

// ensure returns the IDs of all child items of the city keyed by c.keys().
// Children that are not referenced by the city item yet are looked up by their city field first
// and created only when they do not exist.
func (c *cityChildren) ensure(ctx context.Context, city *cms.Item, item SetupCSVItem, missing []string) (map[string]string, error) {
	res := map[string]string{}
	for _, k := range c.keys() {
		if !slices.Contains(missing, k) {
			res[k] = cityRef(city, k)
			continue
		}

		id, err := c.find(ctx, k, city.ID)
		if err != nil {
			return nil, err
		}

		if id == "" {
			id, err = c.create(ctx, k, city.ID, item)
			if err != nil {
				return nil, err
			}
		}

		res[k] = id
	}
	return res, nil
}

func (c *cityChildren) find(ctx context.Context, key, cityID string) (string, error) {
	modelID := c.modelIDs[key]
	items, ok := c.itemsByModel[modelID]
	if !ok {
		var err error
		items, err = listModelItems(ctx, c.cms, modelID)
		if err != nil {
			return "", fmt.Errorf("failed to get %s items: %w", key, err)
		}
		c.itemsByModel[modelID] = items
	}

	for _, i := range items {
		if v := i.FieldByKey("city").GetValue().String(); v != nil && *v == cityID {
			return i.ID, nil
		}
	}
	return "", nil
}

func (c *cityChildren) create(ctx context.Context, key, cityID string, item SetupCSVItem) (string, error) {
	var fields, metadataFields []*cms.Field
	switch key {
	case cmsintegrationcommon.RelatedModel:
		fields = (&cmsintegrationcommon.RelatedItem{City: cityID}).CMSItem(c.relatedDataTypes).Fields
	case cmsintegrationcommon.GeospatialjpIndex:
		fields = (&cmsintegrationcommon.GeospatialjpIndexItem{City: cityID}).CMSItem().Fields
	case cmsintegrationcommon.GeospatialjpData:
		fields = (&cmsintegrationcommon.GeospatialjpDataItem{City: cityID}).CMSItem().Fields
	default:
		var status cmsintegrationcommon.ManagementStatus
		if !slices.Contains(item.Features, key) {
			status = cmsintegrationcommon.ManagementStatusSkip
		}

		featureCMSItem := (&cmsintegrationcommon.FeatureItem{
			City:   cityID,
			Status: cmsintegrationcommon.TagFrom(status),
		}).CMSItem()
		fields, metadataFields = featureCMSItem.Fields, featureCMSItem.MetadataFields
	}

	newItem, err := c.cms.CreateItem(ctx, c.modelIDs[key], fields, metadataFields)
	if err != nil {
		return "", fmt.Errorf("failed to create %s item: %w", key, err)
	}
	return newItem.ID, nil
}

// cityRef returns the ID of the child item referenced by the city item.
func cityRef(city *cms.Item, key string) string {
	keys := []string{key}
	if key == cmsintegrationcommon.RelatedModel {
		// the city model of each project names the field either "related" or "related_dataset"
		keys = []string{"related", "related_dataset"}
	}

	for _, k := range keys {
		if v := city.FieldByKey(k).GetValue().String(); v != nil && *v != "" {
			return *v
		}
	}
	return ""
}

func listModelItems(ctx context.Context, c cms.Interface, modelID string) ([]*cms.Item, error) {
	const perPage = 100
	var res []*cms.Item
	for page := 1; ; page++ {
		items, err := c.GetItemsPartially(ctx, modelID, page, perPage, false)
		if err != nil {
			return nil, err
		}
		for i := range items.Items {
			res = append(res, &items.Items[i])
		}
		if len(items.Items) < perPage || len(res) >= items.TotalCount {
			return res, nil
		}
	}
}

func getAndParseSetupCSV(ctx context.Context, s *Services, url string, body io.Reader) ([]SetupCSVItem, []string, error) {
	if body != nil {
		return parseSetupCSV(body)
	}

	r, err := s.GET(ctx, url)
	if err != nil {
		return nil, nil, err
	}
	defer func() { _ = r.Close() }()

	return parseSetupCSV(r)
}

func parseSetupCSV(r io.Reader) ([]SetupCSVItem, []string, error) {
	cr := csv.NewReader(r)
	cr.ReuseRecord = true

	// read header
	header, err := cr.Read()
	if err != nil {
		return nil, nil, err
	}
	header = slices.Clone(header)
	for i, h := range header {
		header[i], _ = normalizeCSVCell(h)
	}

	nameIndex := -1
	nameEnIndex := -1
	prefectureIndex := -1
	codeIndex := -1
	for i, h := range header {
		switch h {
		case "pref":
			prefectureIndex = i
		case "city":
			nameIndex = i
		case "city_en":
			nameEnIndex = i
		case "city_code":
			codeIndex = i
		}
	}
	if nameIndex == -1 || nameEnIndex == -1 || prefectureIndex == -1 {
		return nil, nil, fmt.Errorf("invalid header: %v", header)
	}

	columnFeaturesIndex := max(codeIndex, nameEnIndex, prefectureIndex) + 1
	features := make([]string, 0, len(header)-3)
	for i := columnFeaturesIndex; i < len(header); i++ {
		features = append(features, header[i])
	}

	var items []SetupCSVItem
	var errs []error
	for line := 2; ; line++ {
		row, err := cr.Read()
		if err == io.EOF {
			break
		}
		if err != nil {
			return nil, nil, err
		}

		var fixes []string
		for i, v := range row {
			// check utf8
			if !utf8.ValidString(v) {
				return nil, nil, fmt.Errorf("invalid utf8 at line %d, column %d", line, i+1)
			}

			nv, fixed := normalizeCSVCell(v)
			if fixed {
				fixes = append(fixes, fmt.Sprintf("%s: removed invisible characters", header[i]))
			}
			row[i] = nv
		}

		if row[nameIndex] == "" {
			return nil, nil, fmt.Errorf("empty city name at line %d", line)
		}

		if row[prefectureIndex] == "" {
			return nil, nil, fmt.Errorf("empty pref name at line %d", line)
		}

		var code string
		if codeIndex == -1 || row[codeIndex] == "" {
			pref := jpareacode.PrefectureCodeInt(row[prefectureIndex])
			city := jpareacode.CityByName(pref, row[nameIndex], "")
			if city == nil {
				return nil, nil, fmt.Errorf("invalid pref or city name: %s", row[nameIndex])
			}

			code = jpareacode.FormatCityCode(city.Code())
		} else {
			code = row[codeIndex]
			// spreadsheet apps drop the leading zero of codes such as 02321
			if len(code) < 5 && isDigits(code) {
				padded := strings.Repeat("0", 5-len(code)) + code
				fixes = append(fixes, fmt.Sprintf("city_code: %s -> %s", code, padded))
				code = padded
			}

			if err := validateCityCode(code, row[prefectureIndex], row[nameIndex]); err != nil {
				errs = append(errs, fmt.Errorf("line %d: %w", line, err))
			}
		}

		itemFeatures := make([]string, 0, len(row)-3)
		for i := columnFeaturesIndex; i < len(row); i++ {
			if row[i] != "" {
				itemFeatures = append(itemFeatures, features[i-columnFeaturesIndex])
			}
		}

		items = append(items, SetupCSVItem{
			Name:       row[nameIndex],
			NameEn:     row[nameEnIndex],
			Code:       code,
			Prefecture: row[prefectureIndex],
			Features:   itemFeatures,
			Fixes:      fixes,
		})
	}

	if len(errs) > 0 {
		return nil, nil, fmt.Errorf("invalid city codes:\n%w", errors.Join(errs...))
	}

	return items, features, nil
}

// normalizeCSVCell removes invisible characters such as zero-width spaces and surrounding white space.
// The second value reports whether invisible characters were removed.
func normalizeCSVCell(v string) (string, bool) {
	nv := strings.Map(func(r rune) rune {
		switch r {
		case '\u200b', '\u200c', '\u200d', '\u2060', '\ufeff':
			return -1
		}
		return r
	}, v)
	return strings.TrimSpace(nv), nv != v
}

// validateCityCode checks that the city code exists and belongs to the prefecture and the city name.
func validateCityCode(code, pref, name string) error {
	city := jpareacode.CityByCodeString(code)
	if city == nil || !isDigits(code) {
		return fmt.Errorf("%s %s: unknown city code %s", pref, name, code)
	}

	if city.PrefCode != jpareacode.PrefectureCodeInt(pref) || (name != city.CityName && name != city.WardName && name != city.CityName+city.WardName) {
		return fmt.Errorf("%s %s: city code %s is %s%s", pref, name, code, city.CityName, city.WardName)
	}

	return nil
}

func isDigits(s string) bool {
	if s == "" {
		return false
	}
	for _, r := range s {
		if r < '0' || r > '9' {
			return false
		}
	}
	return true
}
