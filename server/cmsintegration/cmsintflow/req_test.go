package cmsintflow

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"testing"

	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
	"github.com/jarcoal/httpmock"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestSendRequestToFlow_qc(t *testing.T) {
	ctx := context.Background()
	httpmock.Activate()
	defer httpmock.DeactivateAndReset()

	// cms
	projectID := "projectID"
	modelName := "modelName"

	items := map[string]*cms.Item{
		"main": {
			Fields: []*cms.Field{
				{Key: "city", Value: "cityID"},
				{Key: "citygml", Value: "citygmlID"},
				{Key: "feature_type", Value: "bldg"},
			},
			MetadataFields: []*cms.Field{},
		},
		"cityID": {
			Fields: []*cms.Field{
				{Key: "spec", Value: "v4.1"},
				{Key: "schemas", Value: &cms.Asset{URL: "https://example.com/schemas"}},
				{Key: "codelists", Value: &cms.Asset{URL: "https://example.com/codelists"}},
				{Key: "objectLists", Value: &cms.Asset{URL: "https://example.com/objectLists"}},
				{Key: "prcs", Value: "第1系"}, // 6669
			},
			MetadataFields: []*cms.Field{},
		},
	}

	assets := map[string]*cms.Asset{
		"citygmlID": {
			URL: "https://example.com/citygml",
		},
	}

	featureTypes := plateaucms.PlateauFeatureTypeList{
		{
			Code:   "bldg",
			Name:   "建築物モデル",
			QC:     true,
			FlowQC: "qcTriggerID",
		},
	}

	conf := &Config{
		FlowToken: "token",
	}

	// flow
	httpmock.RegisterResponder("POST", "https://flow.example.com/api/triggers/qcTriggerID/run", func(r *http.Request) (*http.Response, error) {
		t.Helper()

		var body map[string]any
		require.NoError(t, json.NewDecoder(r.Body).Decode(&body))

		assert.Equal(t, map[string]any{
			"cityGmlPath":    "https://example.com/citygml",
			"targetPackages": []any{"bldg"},
			"schemas":        "https://example.com/schemas",
			"codelists":      "https://example.com/codelists",
			"objectLists":    "https://example.com/objectLists",
			"prcs":           "6669",
		}, body["with"])
		assert.Equal(t, "token", body["authToken"])
		assert.NotEmpty(t, body["notificationURL"])

		return httpmock.NewJsonResponse(200, map[string]any{})
	})

	// exec
	s := &Services{
		CMS:  initMockCMS(items, assets),
		Flow: NewFlow(nil, "https://flow.example.com", "token"),
	}
	err := sendRequestToFlow(ctx, s, conf, projectID, modelName, items["main"], featureTypes, "")

	// assert
	assert.NoError(t, err)
	assert.Equal(t, 1, httpmock.GetTotalCallCount())
}

func TestSendRequestToFlow_conv(t *testing.T) {
	ctx := context.Background()
	httpmock.Activate()
	defer httpmock.DeactivateAndReset()

	// cms
	projectID := "projectID"
	modelName := "modelName"

	items := map[string]*cms.Item{
		"main": {
			Fields: []*cms.Field{
				{Key: "city", Value: "cityID"},
				{Key: "citygml", Value: "citygmlID"},
				{Key: "feature_type", Value: "bldg"},
			},
			MetadataFields: []*cms.Field{},
		},
		"cityID": {
			Fields: []*cms.Field{
				{Key: "spec", Value: "v4.1"},
				{Key: "schemas", Value: &cms.Asset{URL: "https://example.com/schemas"}},
				{Key: "codelists", Value: &cms.Asset{URL: "https://example.com/codelists"}},
			},
			MetadataFields: []*cms.Field{},
		},
	}

	assets := map[string]*cms.Asset{
		"citygmlID": {
			URL: "https://example.com/citygml",
		},
	}

	featureTypes := plateaucms.PlateauFeatureTypeList{
		{
			Code:     "bldg",
			Name:     "建築物モデル",
			Conv:     true,
			FlowConv: "convTriggerID",
		},
	}

	conf := &Config{
		FlowToken: "token",
	}

	// flow
	httpmock.RegisterResponder("POST", "https://flow.example.com/api/triggers/convTriggerID/run", func(r *http.Request) (*http.Response, error) {
		t.Helper()

		var body map[string]any
		require.NoError(t, json.NewDecoder(r.Body).Decode(&body))

		assert.Equal(t, map[string]any{
			"cityGmlPath":    "https://example.com/citygml",
			"targetPackages": []any{"bldg"},
			"schemas":        "https://example.com/schemas",
			"codelists":      "https://example.com/codelists",
		}, body["with"])
		assert.Equal(t, "token", body["authToken"])
		assert.NotEmpty(t, body["notificationURL"])

		return httpmock.NewJsonResponse(200, map[string]any{})
	})

	// exec
	s := &Services{
		CMS:  initMockCMS(items, assets),
		Flow: NewFlow(nil, "https://flow.example.com", "token"),
	}
	err := sendRequestToFlow(ctx, s, conf, projectID, modelName, items["main"], featureTypes, cmsintegrationcommon.ReqTypeConv)

	// assert
	assert.NoError(t, err)
	assert.Equal(t, 1, httpmock.GetTotalCallCount())
}

func TestSendRequestToFlow_qc_unsupported(t *testing.T) {
	ctx := context.Background()
	httpmock.Activate()
	defer httpmock.DeactivateAndReset()

	// cms
	projectID := "projectID"
	modelName := "modelName"

	item := &cms.Item{
		Fields: []*cms.Field{
			{Key: "city", Value: "cityID"},
			{Key: "citygml", Value: "citygmlID"},
			{Key: "feature_type", Value: "bldg"},
		},
		MetadataFields: []*cms.Field{},
	}

	featureTypes := plateaucms.PlateauFeatureTypeList{
		{
			Code: "bldg",
			Name: "建築物モデル",
			Conv: false, // unsupported
		},
	}

	// exec
	s := &Services{
		CMS: initMockCMS(nil, nil),
	}
	err := sendRequestToFlow(ctx, s, nil, projectID, modelName, item, featureTypes, "")

	// assert
	assert.NoError(t, err)
	assert.Equal(t, 0, httpmock.GetTotalCallCount())
}

func TestSendRequestToFlow_conv_running(t *testing.T) {
	ctx := context.Background()
	httpmock.Activate()
	defer httpmock.DeactivateAndReset()

	// cms
	projectID := "projectID"
	modelName := "modelName"

	item := &cms.Item{
		Fields: []*cms.Field{
			{Key: "city", Value: "cityID"},
			{Key: "citygml", Value: "citygmlID"},
			{Key: "feature_type", Value: "bldg"},
		},
		MetadataFields: []*cms.Field{
			{Key: "qc_status", Value: &cms.Tag{Name: "成功"}},
			{Key: "conv_status", Value: &cms.Tag{Name: "実行中"}},
		},
	}

	featureTypes := plateaucms.PlateauFeatureTypeList{
		{
			Code: "bldg",
			Name: "建築物モデル",
			Conv: true,
		},
	}

	// exec
	s := &Services{
		CMS: initMockCMS(nil, nil),
	}
	err := sendRequestToFlow(ctx, s, nil, projectID, modelName, item, featureTypes, "")

	// assert
	assert.NoError(t, err)
	assert.Equal(t, 0, httpmock.GetTotalCallCount())
}

type mockCMS struct {
	cms.Interface
	items  map[string]*cms.Item
	assets map[string]*cms.Asset
}

func initMockCMS(items map[string]*cms.Item, assets map[string]*cms.Asset) *mockCMS {
	return &mockCMS{items: items, assets: assets}
}

func (m *mockCMS) GetItem(ctx context.Context, itemID string, asset bool) (*cms.Item, error) {
	if item, ok := m.items[itemID]; ok {
		return item, nil
	}
	return nil, errors.New("item not found")
}

func (m *mockCMS) UpdateItem(ctx context.Context, itemID string, fields, mfields []*cms.Field) (*cms.Item, error) {
	return nil, nil
}

func (m *mockCMS) CommentToItem(ctx context.Context, itemID, comment string) error {
	return nil
}

func (m *mockCMS) Asset(ctx context.Context, assetID string) (*cms.Asset, error) {
	if asset, ok := m.assets[assetID]; ok {
		return asset, nil
	}
	return nil, errors.New("asset not found")
}
