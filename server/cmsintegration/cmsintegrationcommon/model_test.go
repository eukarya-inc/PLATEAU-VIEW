package cmsintegrationcommon

import (
	"fmt"
	"testing"

	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/stretchr/testify/assert"
)

func TestCityItemFrom(t *testing.T) {
	item := &cms.Item{
		ID: "id",
		Fields: []*cms.Field{
			{
				Key:   "bldg",
				Type:  "reference",
				Value: "BLDG",
			},
		},
		MetadataFields: []*cms.Field{
			{
				Key:   "city_public",
				Type:  "bool",
				Value: true,
			},
			{
				Key:   "bldg_public",
				Type:  "bool",
				Value: true,
			},
		},
	}

	expected := &CityItem{
		ID: "id",
		References: map[string]string{
			"bldg": "BLDG",
		},
		Public: map[string]bool{
			"bldg": true,
		},
		CityPublic: true,
	}

	cityItem := CityItemFrom(item, []string{"bldg"})
	assert.Equal(t, expected, cityItem)
	item2 := cityItem.CMSItem([]string{"bldg"})
	assert.Equal(t, item, item2)
}

func TestFeatureItemFrom(t *testing.T) {
	item := &cms.Item{
		ID: "id",
		MetadataFields: []*cms.Field{
			{
				Key:  "conv_status",
				Type: "tag",
				Value: map[string]any{
					"id":   "xxx",
					"name": string(ConvertionStatusError),
				},
			},
		},
	}

	expected := &FeatureItem{
		ID: "id",
		ConvertionStatus: &cms.Tag{
			ID:   "xxx",
			Name: string(ConvertionStatusError),
		},
	}

	expected2 := &cms.Item{
		ID: "id",
		MetadataFields: []*cms.Field{
			{
				Key:   "conv_status",
				Type:  "tag",
				Value: "xxx",
			},
		},
	}

	featureItem := FeatureItemFrom(item)
	assert.Equal(t, expected, featureItem)
	item2 := featureItem.CMSItem()
	assert.Equal(t, expected2, item2)
}

func TestFeatureItem_FeatureTypeCode(t *testing.T) {
	assert.Equal(t, "", (&FeatureItem{FeatureType: ""}).FeatureTypeCode())
	assert.Equal(t, "bldg", (&FeatureItem{FeatureType: "bldg"}).FeatureTypeCode())
	assert.Equal(t, "bldg", (&FeatureItem{FeatureType: "建築物モデル（bldg）"}).FeatureTypeCode())
	assert.Equal(t, "bldg", (&FeatureItem{FeatureType: "建築物モデル (bldg)"}).FeatureTypeCode())
}

func TestGenericItemFrom(t *testing.T) {
	item := &cms.Item{
		ID: "id",
		MetadataFields: []*cms.Field{
			{
				Key:   "public",
				Type:  "bool",
				Value: true,
			},
		},
	}

	expected := &GenericItem{
		ID:     "id",
		Public: true,
	}

	expected2 := &cms.Item{
		ID: "id",
		MetadataFields: []*cms.Field{
			{
				Key:   "public",
				Type:  "bool",
				Value: true,
			},
		},
	}

	genericItem := GenericItemFrom(item)
	assert.Equal(t, expected, genericItem)
	item2 := genericItem.CMSItem()
	assert.Equal(t, expected2, item2)
}

func TestRelatedItemFrom(t *testing.T) {
	item := &cms.Item{
		ID: "id",
		Fields: []*cms.Field{
			{
				Key:   "asset",
				Type:  "asset",
				Value: []string{"PARK"},
				Group: "park",
			},
			{
				Key:   "conv",
				Type:  "asset",
				Value: []string{"PARK_CONV"},
				Group: "park",
			},
			{
				Key:   "park",
				Type:  "group",
				Value: "park",
			},
			{
				Key:   "asset",
				Type:  "asset",
				Value: []string{"LANDMARK"},
				Group: "landmark",
			},
			{
				Key:   "landmark",
				Type:  "group",
				Value: "landmark",
			},
		},
		MetadataFields: []*cms.Field{
			{
				Key:   "park_status",
				Type:  "tag",
				Value: map[string]any{"id": "xxx", "name": string(ConvertionStatusSuccess)},
			},
			{
				Key:   "merge_status",
				Type:  "tag",
				Value: map[string]any{"id": "xxx", "name": string(ConvertionStatusSuccess)},
			},
		},
	}

	expected := &RelatedItem{
		ID: "id",
		Items: map[string]RelatedItemDatum{
			"park": {
				ID:        "park",
				Asset:     []string{"PARK"},
				Converted: []string{"PARK_CONV"},
			},
			"landmark": {
				ID:    "landmark",
				Asset: []string{"LANDMARK"},
			},
		},
		ConvertStatus: map[string]*cms.Tag{
			"park": {
				ID:   "xxx",
				Name: string(ConvertionStatusSuccess),
			},
		},
		MergeStatus: &cms.Tag{
			ID:   "xxx",
			Name: string(ConvertionStatusSuccess),
		},
	}

	expected2 := &cms.Item{
		ID: "id",
		Fields: []*cms.Field{
			{
				Key:   "asset",
				Type:  "asset",
				Value: []string{"PARK"},
				Group: "park",
			},
			{
				Key:   "conv",
				Type:  "asset",
				Value: []string{"PARK_CONV"},
				Group: "park",
			},
			{
				Key:   "park",
				Type:  "group",
				Value: "park",
			},
			{
				Key:   "asset",
				Type:  "asset",
				Value: []string{"LANDMARK"},
				Group: "landmark",
			},
			{
				Key:   "landmark",
				Type:  "group",
				Value: "landmark",
			},
		},
		MetadataFields: []*cms.Field{
			{
				Key:   "merge_status",
				Type:  "tag",
				Value: "xxx",
			},
			{
				Key:   "park_status",
				Type:  "tag",
				Value: string(ConvertionStatusSuccess),
			},
		},
	}

	relatedItem := RelatedItemFrom(item, []string{"park", "landmark"})
	assert.Equal(t, expected, relatedItem)
	item2 := relatedItem.CMSItem([]string{"park", "landmark"})
	assert.Equal(t, expected2, item2)
}

func TestCityItem_SpecMajorVersionInt(t *testing.T) {
	assert.Equal(t, 4, (&CityItem{Spec: "第4版"}).SpecMajorVersionInt())
	assert.Equal(t, 4, (&CityItem{Spec: "4版"}).SpecMajorVersionInt())
	assert.Equal(t, 4, (&CityItem{Spec: "v4"}).SpecMajorVersionInt())
	assert.Equal(t, 4, (&CityItem{Spec: "第4.2版"}).SpecMajorVersionInt())
	assert.Equal(t, 4, (&CityItem{Spec: "4.2版"}).SpecMajorVersionInt())
	assert.Equal(t, 4, (&CityItem{Spec: "v4.2"}).SpecMajorVersionInt())
}

func TestFeatureItem_SpecMajorVersionInt(t *testing.T) {
	assert.Equal(t, 0, (&FeatureItem{}).SpecMajorVersionInt())
	assert.Equal(t, 0, (&FeatureItem{Spec: ""}).SpecMajorVersionInt())
	assert.Equal(t, 4, (&FeatureItem{Spec: "第4版"}).SpecMajorVersionInt())
	assert.Equal(t, 4, (&FeatureItem{Spec: "4版"}).SpecMajorVersionInt())
	assert.Equal(t, 4, (&FeatureItem{Spec: "v4"}).SpecMajorVersionInt())
	assert.Equal(t, 4, (&FeatureItem{Spec: "第4.2版"}).SpecMajorVersionInt())
	assert.Equal(t, 4, (&FeatureItem{Spec: "4.2版"}).SpecMajorVersionInt())
	assert.Equal(t, 4, (&FeatureItem{Spec: "v4.2"}).SpecMajorVersionInt())
}

func TestIsQCAndConvSkipped(t *testing.T) {
	skipQC, skipConv := (&FeatureItem{}).IsQCAndConvSkipped()
	assert.False(t, skipQC)
	assert.False(t, skipConv)

	skipQC, skipConv = (&FeatureItem{
		QCStatus: &cms.Tag{
			Name: "成功",
		},
	}).IsQCAndConvSkipped()
	assert.True(t, skipQC)
	assert.False(t, skipConv)

	skipQC, skipConv = (&FeatureItem{
		ConvertionStatus: &cms.Tag{
			Name: "成功",
		},
	}).IsQCAndConvSkipped()
	assert.False(t, skipQC)
	assert.True(t, skipConv)

	skipQC, skipConv = (&FeatureItem{
		SkipQCConv: &cms.Tag{
			Name: "品質検査のみをスキップ",
		},
	}).IsQCAndConvSkipped()
	assert.True(t, skipQC)
	assert.False(t, skipConv)

	skipQC, skipConv = (&FeatureItem{
		SkipQCConv: &cms.Tag{
			Name: "変換のみをスキップ",
		},
	}).IsQCAndConvSkipped()
	assert.False(t, skipQC)
	assert.True(t, skipConv)

	skipQC, skipConv = (&FeatureItem{
		SkipQCConv: &cms.Tag{
			Name: "品質検査・変換のみをスキップ",
		},
	}).IsQCAndConvSkipped()
	assert.True(t, skipQC)
	assert.True(t, skipConv)

	skipQC, skipConv = (&FeatureItem{
		SkipQC: true,
	}).IsQCAndConvSkipped()
	assert.True(t, skipQC)
	assert.False(t, skipConv)

	skipQC, skipConv = (&FeatureItem{
		SkipConvert: true,
	}).IsQCAndConvSkipped()
	assert.False(t, skipQC)
	assert.True(t, skipConv)

	skipQC, skipConv = (&FeatureItem{
		SkipQC:      true,
		SkipConvert: true,
	}).IsQCAndConvSkipped()
	assert.True(t, skipQC)
	assert.True(t, skipConv)
}

func TestReqType_Intersection(t *testing.T) {
	tests := []struct {
		input    ReqType
		other    ReqType
		expected ReqType
	}{
		{input: ReqTypeQC, other: ReqTypeQC, expected: ReqTypeQC},
		{input: ReqTypeQC, other: ReqTypeQCConv, expected: ReqTypeQC},
		{input: ReqTypeQC, other: ReqTypeConv, expected: ""},
		{input: ReqTypeConv, other: ReqTypeConv, expected: ReqTypeConv},
		{input: ReqTypeConv, other: ReqTypeQCConv, expected: ReqTypeConv},
		{input: ReqTypeConv, other: ReqTypeQC, expected: ""},
		{input: ReqTypeQCConv, other: ReqTypeQCConv, expected: ReqTypeQCConv},
		{input: ReqTypeQCConv, other: ReqTypeQC, expected: ReqTypeQC},
		{input: ReqTypeQCConv, other: ReqTypeConv, expected: ReqTypeConv},
		{input: ReqTypeQCConv, other: "", expected: ""},
		{input: "", other: ReqTypeQCConv, expected: ""},
	}

	for _, tt := range tests {
		name := fmt.Sprintf("%s and %s", tt.input, tt.other)
		t.Run(name, func(t *testing.T) {
			result := tt.input.Intersection(tt.other)
			assert.Equal(t, tt.expected, result)
		})
	}
}

func TestReqType_Normalize(t *testing.T) {
	tests := []struct {
		input    ReqType
		expected ReqType
	}{
		{input: ReqTypeQC, expected: ReqTypeQC},
		{input: ReqTypeConv, expected: ReqTypeConv},
		{input: ReqTypeQCConv, expected: ReqTypeQC},
		{input: "", expected: ""},
	}

	for _, tt := range tests {
		t.Run(string(tt.input), func(t *testing.T) {
			result := tt.input.Normalize()
			assert.Equal(t, tt.expected, result)
		})
	}
}

func TestGetEffectiveConverter(t *testing.T) {
	tests := []struct {
		name          string
		featureItem   *FeatureItem
		cityItem      *CityItem
		specConverter string
		want          string
	}{
		{
			name:          "all nil/empty: returns spec converter",
			featureItem:   nil,
			cityItem:      nil,
			specConverter: "fme",
			want:          "fme",
		},
		{
			name:          "spec converter only",
			featureItem:   &FeatureItem{},
			cityItem:      &CityItem{},
			specConverter: "flow",
			want:          "flow",
		},
		{
			name:          "city item overrides spec",
			featureItem:   nil,
			cityItem:      &CityItem{Converter: "fme_flow"},
			specConverter: "fme",
			want:          "fme_flow",
		},
		{
			name:          "feature item overrides city item",
			featureItem:   &FeatureItem{Converter: "flow"},
			cityItem:      &CityItem{Converter: "fme_flow"},
			specConverter: "fme",
			want:          "flow",
		},
		{
			name:          "feature item overrides spec (city item nil)",
			featureItem:   &FeatureItem{Converter: "flow"},
			cityItem:      nil,
			specConverter: "fme",
			want:          "flow",
		},
		{
			name:          "feature item empty: falls back to city item",
			featureItem:   &FeatureItem{Converter: ""},
			cityItem:      &CityItem{Converter: "fme_flow"},
			specConverter: "fme",
			want:          "fme_flow",
		},
		{
			name:          "both items empty: falls back to spec",
			featureItem:   &FeatureItem{Converter: ""},
			cityItem:      &CityItem{Converter: ""},
			specConverter: "flow",
			want:          "flow",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := GetEffectiveConverter(tt.featureItem, tt.cityItem, tt.specConverter)
			assert.Equal(t, tt.want, got)
		})
	}
}
