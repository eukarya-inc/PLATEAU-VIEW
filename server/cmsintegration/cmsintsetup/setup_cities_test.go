package cmsintsetup

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"strings"
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	"github.com/jarcoal/httpmock"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/stretchr/testify/assert"
)

func TestParseSetupCSV(t *testing.T) {
	tests := []struct {
		name             string
		csvData          string
		expected         []SetupCSVItem
		expectedFeatures []string
		err              error
	}{
		{
			name: "valid csv",
			csvData: `city_code,city,city_en,pref,Feature1,Feature2,Feature3
13201,八王子市,hachioji-shi,東京都,Yes,,Yes
13213,東村山市,higashimurayama-shi,東京都,Yes,Yes,`,
			expected: []SetupCSVItem{
				{
					Name:       "八王子市",
					NameEn:     "hachioji-shi",
					Code:       "13201",
					Prefecture: "東京都",
					Features:   []string{"Feature1", "Feature3"},
				},
				{
					Name:       "東村山市",
					NameEn:     "higashimurayama-shi",
					Code:       "13213",
					Prefecture: "東京都",
					Features:   []string{"Feature1", "Feature2"},
				},
			},
			expectedFeatures: []string{"Feature1", "Feature2", "Feature3"},
			err:              nil,
		},
		{
			name: "normalized csv",
			csvData: "\ufeffcity_code,city,city_en,pref,Feature1\n" +
				"2321,鰺ヶ沢町,ajigasawa-machi,青森県,LOD2\n" +
				"39201,高知市\u200b, kochi-shi ,高知県,\n",
			expected: []SetupCSVItem{
				{
					Name:       "鰺ヶ沢町",
					NameEn:     "ajigasawa-machi",
					Code:       "02321",
					Prefecture: "青森県",
					Features:   []string{"Feature1"},
					Fixes:      []string{"city_code: 2321 -> 02321"},
				},
				{
					Name:       "高知市",
					NameEn:     "kochi-shi",
					Code:       "39201",
					Prefecture: "高知県",
					Features:   []string{},
					Fixes:      []string{"city: removed invisible characters"},
				},
			},
			expectedFeatures: []string{"Feature1"},
		},
		{
			name: "mismatched city codes",
			csvData: `city_code,city,city_en,pref,Feature1
13201,東村山市,higashimurayama-shi,東京都,
13213,東村山市,higashimurayama-shi,東京都,
99999,八王子市,hachioji-shi,東京都,
13201,八王子市,hachioji-shi,神奈川県,`,
			err: fmt.Errorf("invalid city codes:\n" +
				"line 2: 東京都 東村山市: city code 13201 is 八王子市\n" +
				"line 4: 東京都 八王子市: unknown city code 99999\n" +
				"line 5: 神奈川県 八王子市: city code 13201 is 八王子市"),
		},
		{
			name:             "empty csv",
			csvData:          "",
			expected:         nil,
			expectedFeatures: nil,
			err:              io.EOF,
		},
		{
			name: "invalid header",
			csvData: `city,city_en
八王子市,hachioji-shi`,
			expected:         nil,
			expectedFeatures: nil,
			err:              fmt.Errorf("invalid header: [city city_en]"),
		},
		{
			name: "invalid row",
			csvData: `city,city_en,pref,Feature1
八王子市,hachioji-shi,東京都`,
			expected: nil,
			err:      fmt.Errorf("record on line 2: wrong number of fields"),
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := strings.NewReader(tt.csvData)
			items, features, err := parseSetupCSV(r)

			if tt.err != nil {
				assert.EqualError(t, err, tt.err.Error())
			} else {
				assert.NoError(t, err)
			}

			assert.Equal(t, tt.expected, items)
			assert.Equal(t, tt.expectedFeatures, features)
		})
	}
}

func TestSetupCityItems(t *testing.T) {
	ctx := context.Background()
	httpmock.Activate()
	defer httpmock.DeactivateAndReset()

	httpmock.RegisterResponder("GET", "https://example.com/data.csv",
		httpmock.NewStringResponder(200, `city,city_en,pref,bldg,tran,luse
八王子市,hachioji-shi,東京都,Yes,,Yes
東村山市,higashimurayama-shi,東京都,Yes,Yes,`))

	var createdItems []*cms.Item
	var updateditems []*cms.Item

	s := &Services{
		CMS: &cmsintegrationcommon.CMSMock{
			MockGetModels: func(ctx context.Context, projectID string) (*cms.Models, error) {
				return &cms.Models{
					Models: []cms.Model{
						{
							ID:  "city",
							Key: "plateau-city",
						},
						{
							ID:  "bldg",
							Key: "plateau-bldg",
						},
						{
							ID:  "tran",
							Key: "plateau-tran",
						},
						{
							ID:  "luse",
							Key: "plateau-luse",
						},
						{
							ID:  "related",
							Key: "plateau-related",
						},
						{
							ID:  "geospatialjp-index",
							Key: "plateau-geospatialjp-index",
						},
						{
							ID:  "geospatialjp-data",
							Key: "plateau-geospatialjp-data",
						},
					},
				}, nil
			},
			MockGetItemsPartially: func(ctx context.Context, modelID string, page, perPage int, asset bool) (*cms.Items, error) {
				return &cms.Items{
					TotalCount: 0,
				}, nil
			},
			MockCreateItem: func(ctx context.Context, modelID string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
				item := &cms.Item{
					ID:             fmt.Sprintf("item%d", len(createdItems)),
					ModelID:        modelID,
					Fields:         fields,
					MetadataFields: metadataFields,
				}
				createdItems = append(createdItems, item)
				return item, nil
			},
			MockUpdateItem: func(ctx context.Context, itemID string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
				item := &cms.Item{
					ID:             itemID,
					Fields:         fields,
					MetadataFields: metadataFields,
				}
				updateditems = append(updateditems, item)
				return item, nil
			},
		},
		PCMS: &plateauCMSMock{},
		HTTP: http.DefaultClient,
	}

	inp := SetupCityItemsInput{
		ProjectID: "project123",
		DataURL:   "https://example.com/data.csv",
	}

	onprogress := func(i, l int, c SetupCSVItem, action SetupAction) {}

	t.Run("success", func(t *testing.T) {
		createdItems = nil
		updateditems = nil
		res, err := SetupCityItems(ctx, s, inp, onprogress)
		assert.NoError(t, err)
		assert.Equal(t, []string{"八王子市", "東村山市"}, res.Created)

		assert.Equal(t, 14, len(createdItems))
		assertCityItem(t, &cmsintegrationcommon.CityItem{
			ID:         "item0",
			CityName:   "八王子市",
			CityNameEn: "hachioji-shi",
			CityCode:   "13201",
			Prefecture: "東京都",
		}, createdItems[0])
		assertFeatureItem(t, "related", "item0", "", createdItems[1])
		assertFeatureItem(t, "geospatialjp-index", "item0", "", createdItems[2])
		assertFeatureItem(t, "geospatialjp-data", "item0", "", createdItems[3])
		assertFeatureItem(t, "bldg", "item0", "", createdItems[4])
		assertFeatureItem(t, "tran", "item0", cmsintegrationcommon.ManagementStatusSkip, createdItems[5])
		assertFeatureItem(t, "luse", "item0", "", createdItems[6])
		assertCityItem(t, &cmsintegrationcommon.CityItem{
			ID:         "item7",
			CityName:   "東村山市",
			CityNameEn: "higashimurayama-shi",
			CityCode:   "13213",
			Prefecture: "東京都",
		}, createdItems[7])
		assertFeatureItem(t, "related", "item7", "", createdItems[8])
		assertFeatureItem(t, "geospatialjp-index", "item7", "", createdItems[9])
		assertFeatureItem(t, "geospatialjp-data", "item7", "", createdItems[10])
		assertFeatureItem(t, "bldg", "item7", "", createdItems[11])
		assertFeatureItem(t, "tran", "item7", "", createdItems[12])
		assertFeatureItem(t, "luse", "item7", cmsintegrationcommon.ManagementStatusSkip, createdItems[13])

		assert.Equal(t, 2, len(updateditems))
		assertUpdatedCityItem(t, &cmsintegrationcommon.CityItem{
			ID: "item0",
			References: map[string]string{
				"bldg": "item4",
				"tran": "item5",
				"luse": "item6",
			},
			RelatedDataset:    "item1",
			GeospatialjpIndex: "item2",
			GeospatialjpData:  "item3",
		}, updateditems[0])
		assertUpdatedCityItem(t, &cmsintegrationcommon.CityItem{
			ID: "item7",
			References: map[string]string{
				"bldg": "item11",
				"tran": "item12",
				"luse": "item13",
			},
			RelatedDataset:    "item8",
			GeospatialjpIndex: "item9",
			GeospatialjpData:  "item10",
		}, updateditems[1])
	})
}

func TestSetupCityItems_SkipAndResume(t *testing.T) {
	ctx := context.Background()

	csvData := `city_code,city,city_en,pref,bldg,tran
13201,八王子市,hachioji-shi,東京都,LOD2,
13213,東村山市,higashimurayama-shi,東京都,LOD2,LOD1
13219,狛江市,komae-shi,東京都,LOD2,`

	ref := func(key, value string) *cms.Field {
		return &cms.Field{Key: key, Type: "reference", Value: value}
	}
	cityCode := func(code string) *cms.Field {
		return &cms.Field{Key: "city_code", Type: "text", Value: code}
	}

	existing := map[string][]cms.Item{
		"city": {
			// complete
			{ID: "hachioji", Fields: []*cms.Field{
				cityCode("13201"),
				ref("related", "r-hachioji"),
				ref("geospatialjp-index", "gi-hachioji"),
				ref("geospatialjp-data", "gd-hachioji"),
				ref("bldg", "b-hachioji"),
				ref("tran", "t-hachioji"),
			}},
			// stopped in the middle: the related item exists but the city item does not reference it yet
			{ID: "higashimurayama", Fields: []*cms.Field{
				cityCode("13213"),
			}},
			// test item without a city code
			{ID: "test", Fields: []*cms.Field{}},
		},
		"related": {
			{ID: "r-higashimurayama", Fields: []*cms.Field{ref("city", "higashimurayama")}},
		},
	}

	var createdItems []*cms.Item
	var updatedItems []*cms.Item

	s := &Services{
		CMS: &cmsintegrationcommon.CMSMock{
			MockGetModels: func(ctx context.Context, projectID string) (*cms.Models, error) {
				return &cms.Models{
					Models: []cms.Model{
						{ID: "city", Key: "plateau-city"},
						{ID: "bldg", Key: "plateau-bldg"},
						{ID: "tran", Key: "plateau-tran"},
						{ID: "related", Key: "plateau-related"},
						{ID: "geospatialjp-index", Key: "plateau-geospatialjp-index"},
						{ID: "geospatialjp-data", Key: "plateau-geospatialjp-data"},
					},
				}, nil
			},
			MockGetItemsPartially: func(ctx context.Context, modelID string, page, perPage int, asset bool) (*cms.Items, error) {
				items := existing[modelID]
				if page > 1 {
					items = nil
				}
				return &cms.Items{Items: items, TotalCount: len(existing[modelID])}, nil
			},
			MockCreateItem: func(ctx context.Context, modelID string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
				item := &cms.Item{
					ID:             fmt.Sprintf("item%d", len(createdItems)),
					ModelID:        modelID,
					Fields:         fields,
					MetadataFields: metadataFields,
				}
				createdItems = append(createdItems, item)
				return item, nil
			},
			MockUpdateItem: func(ctx context.Context, itemID string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
				item := &cms.Item{ID: itemID, Fields: fields}
				updatedItems = append(updatedItems, item)
				return item, nil
			},
		},
		PCMS: &plateauCMSMock{},
	}

	t.Run("dry run", func(t *testing.T) {
		createdItems, updatedItems = nil, nil
		var actions []SetupAction
		res, err := SetupCityItems(ctx, s, SetupCityItemsInput{
			ProjectID: "project",
			DataBody:  strings.NewReader(csvData),
			DryRun:    true,
		}, func(i, l int, c SetupCSVItem, action SetupAction) {
			actions = append(actions, action)
		})
		assert.NoError(t, err)
		assert.Equal(t, []SetupAction{SetupActionSkip, SetupActionResume, SetupActionCreate}, actions)
		assert.Equal(t, []string{"狛江市"}, res.Created)
		assert.Equal(t, []string{"東村山市"}, res.Resumed)
		assert.Equal(t, []string{"八王子市"}, res.Skipped)
		assert.Empty(t, createdItems)
		assert.Empty(t, updatedItems)
	})

	t.Run("run", func(t *testing.T) {
		createdItems, updatedItems = nil, nil
		_, err := SetupCityItems(ctx, s, SetupCityItemsInput{
			ProjectID: "project",
			DataBody:  strings.NewReader(csvData),
			Limit:     2,
		}, nil)
		assert.NoError(t, err)

		// the related item of 東村山市 is reused, the other children are created
		assert.Equal(t, 4, len(createdItems))
		assertFeatureItem(t, "geospatialjp-index", "higashimurayama", "", createdItems[0])
		assertFeatureItem(t, "geospatialjp-data", "higashimurayama", "", createdItems[1])
		assertFeatureItem(t, "bldg", "higashimurayama", "", createdItems[2])
		assertFeatureItem(t, "tran", "higashimurayama", "", createdItems[3])

		assert.Equal(t, 1, len(updatedItems))
		assertUpdatedCityItem(t, &cmsintegrationcommon.CityItem{
			ID: "higashimurayama",
			References: map[string]string{
				"bldg": "item2",
				"tran": "item3",
			},
			RelatedDataset:    "r-higashimurayama",
			GeospatialjpIndex: "item0",
			GeospatialjpData:  "item1",
		}, updatedItems[0])
	})

	t.Run("offset", func(t *testing.T) {
		createdItems, updatedItems = nil, nil
		var names []string
		_, err := SetupCityItems(ctx, s, SetupCityItemsInput{
			ProjectID: "project",
			DataBody:  strings.NewReader(csvData),
			Offset:    2,
			DryRun:    true,
		}, func(i, l int, c SetupCSVItem, action SetupAction) {
			names = append(names, c.Name)
		})
		assert.NoError(t, err)
		assert.Equal(t, []string{"狛江市"}, names)
	})
}

func assertCityItem(t *testing.T, expected *cmsintegrationcommon.CityItem, actual *cms.Item) {
	t.Helper()
	assert.Equal(t, "city", actual.ModelID)
	a := cmsintegrationcommon.CityItemFrom(actual, []string{"bldg", "tran", "luse"})
	am := &cmsintegrationcommon.CityItem{
		ID:         a.ID,
		CityName:   a.CityName,
		CityNameEn: a.CityNameEn,
		CityCode:   a.CityCode,
		Prefecture: a.Prefecture,
	}
	assert.Equal(t, expected, am)
}

func assertFeatureItem(t *testing.T, expectedModel, expectedCity string, status cmsintegrationcommon.ManagementStatus, actual *cms.Item) {
	t.Helper()
	assert.Equal(t, expectedModel, actual.ModelID, "model of "+actual.ID)
	assert.Equal(t, expectedCity, actual.FieldByKey("city").GetValue().Interface(), "city of "+actual.ID)
	statusv := actual.MetadataFieldByKey("status").GetValue().Interface()
	if status == "" {
		assert.Nil(t, statusv, "status of "+actual.ID)
		return
	}
	assert.Equal(t, string(status), statusv, "status of "+actual.ID)
}

func assertUpdatedCityItem(t *testing.T, expected *cmsintegrationcommon.CityItem, actual *cms.Item) {
	t.Helper()
	a := cmsintegrationcommon.CityItemFrom(actual, []string{"bldg", "tran", "luse"})
	am := &cmsintegrationcommon.CityItem{
		ID:                a.ID,
		References:        a.References,
		RelatedDataset:    a.RelatedDataset,
		GeospatialjpIndex: a.GeospatialjpIndex,
		GeospatialjpData:  a.GeospatialjpData,
	}
	assert.Equal(t, expected, am)
}
