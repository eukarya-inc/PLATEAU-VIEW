package cmsintflow

import (
	"context"
	"errors"
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/jarcoal/httpmock"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

type reconvertPCMS struct {
	PCMS
	featureTypes plateaucms.PlateauFeatureTypeList
	specs        []plateaucms.PlateauSpec
}

func (p reconvertPCMS) PlateauFeatureTypes(context.Context) (plateaucms.PlateauFeatureTypeList, error) {
	return p.featureTypes, nil
}

func (p reconvertPCMS) PlateauSpecs(context.Context) ([]plateaucms.PlateauSpec, error) {
	return p.specs, nil
}

// reconvertCMS keeps the metadata item, so that status updates are seen by later reads.
type reconvertCMS struct {
	cms.Interface
	items  map[string]*cms.Item
	assets map[string]*cms.Asset
	// statuses records conv_status of every metadata update in order.
	statuses []string
}

func (m *reconvertCMS) GetItem(ctx context.Context, id string, asset bool) (*cms.Item, error) {
	if i, ok := m.items[id]; ok {
		c := *i
		c.Fields = append([]*cms.Field(nil), i.Fields...)
		c.MetadataFields = append([]*cms.Field(nil), i.MetadataFields...)
		return &c, nil
	}
	return nil, errors.New("not found")
}

func (m *reconvertCMS) UpdateItem(ctx context.Context, id string, fields, metadataFields []*cms.Field) (*cms.Item, error) {
	item := m.items[id]
	for _, f := range metadataFields {
		if f.Key == "conv_status" {
			if name, ok := f.Value.(string); ok {
				m.statuses = append(m.statuses, name)
				setMetadataTag(item, "conv_status", name)
			}
		}
	}
	return nil, nil
}

func (m *reconvertCMS) CommentToItem(ctx context.Context, id, comment string) error {
	return nil
}

func (m *reconvertCMS) Asset(ctx context.Context, id string) (*cms.Asset, error) {
	if a, ok := m.assets[id]; ok {
		return a, nil
	}
	return nil, errors.New("not found")
}

// newReconvertFixture returns an item in the shape the integration API returns it: the metadata
// is in metadataFields and there is no metadata item ID.
func newReconvertFixture(convStatus string, converter string) (*Services, *reconvertCMS) {
	var metaFields []*cms.Field
	if convStatus != "" {
		metaFields = append(metaFields, &cms.Field{Key: "conv_status", Type: "tag", Value: map[string]any{"name": convStatus}})
	}
	c := &reconvertCMS{
		items: map[string]*cms.Item{
			"main": {
				ID: "main",
				Fields: []*cms.Field{
					{Key: "city", Value: "city"},
					{Key: "citygml", Value: "citygml"},
				},
				MetadataFields: metaFields,
			},
			"city": {
				Fields: []*cms.Field{
					{Key: "spec", Value: "v4.1"},
					{Key: "schemas", Value: &cms.Asset{URL: "https://example.com/schemas"}},
					{Key: "codelists", Value: &cms.Asset{URL: "https://example.com/codelists"}},
				},
			},
		},
		assets: map[string]*cms.Asset{"citygml": {URL: "https://example.com/citygml.zip"}},
	}
	s := &Services{
		CMS:  c,
		Flow: NewFlow(nil, "https://flow.example.com", "token"),
		PCMS: reconvertPCMS{
			featureTypes: plateaucms.PlateauFeatureTypeList{{Code: "bldg", QC: true, Conv: true}},
			specs: []plateaucms.PlateauSpec{{
				MajorVersion: 4,
				Converter:    converter,
				FlowTriggers: []plateaucms.FlowTrigger{{FeatureType: "bldg", FlowQCTrigger: "qc", FlowConvTrigger: "conv"}},
			}},
		},
	}
	return s, c
}

func TestReconvert_convertedItem(t *testing.T) {
	httpmock.Activate()
	defer httpmock.DeactivateAndReset()
	httpmock.RegisterResponder("POST", "https://flow.example.com/api/triggers/conv/run",
		httpmock.NewJsonResponderOrPanic(200, map[string]any{}))

	s, c := newReconvertFixture("成功", plateaucms.ConverterFlow)
	res, err := reconvert(context.Background(), s, &Config{FlowToken: "token"}, ReconvertRequest{ProjectID: "p", Model: "plateau-bldg", ItemID: "main"})
	require.NoError(t, err)

	// conv_status goes 成功 → 未実行 → 実行中, and only the conversion trigger is called (no QC)
	assert.Equal(t, &ReconvertResult{ItemID: "main", Started: true, ConvStatus: "実行中"}, res)
	assert.Equal(t, []string{"未実行", "実行中"}, c.statuses)
	assert.Equal(t, 1, httpmock.GetTotalCallCount())
}

func TestReconvert_notFlow(t *testing.T) {
	httpmock.Activate()
	defer httpmock.DeactivateAndReset()

	s, c := newReconvertFixture("成功", plateaucms.ConverterFME)
	_, err := reconvert(context.Background(), s, &Config{}, ReconvertRequest{ProjectID: "p", Model: "plateau-bldg", ItemID: "main"})

	// nothing is requested and conv_status is not touched
	assert.ErrorIs(t, err, ErrReconvertNotSupported)
	assert.Empty(t, c.statuses)
	assert.Equal(t, 0, httpmock.GetTotalCallCount())
}

func TestReconvert_flowDisabledForFeature(t *testing.T) {
	s, c := newReconvertFixture("成功", plateaucms.ConverterFMEFlow)
	s.PCMS = reconvertPCMS{
		featureTypes: plateaucms.PlateauFeatureTypeList{{Code: "bldg", QC: true, Conv: true}},
		specs: []plateaucms.PlateauSpec{{
			MajorVersion: 4,
			Converter:    plateaucms.ConverterFMEFlow,
			FlowTriggers: []plateaucms.FlowTrigger{{FeatureType: "bldg", FlowConvTrigger: "conv", FlowDisabled: true}},
		}},
	}
	_, err := reconvert(context.Background(), s, &Config{}, ReconvertRequest{ProjectID: "p", Model: "plateau-bldg", ItemID: "main"})
	assert.ErrorIs(t, err, ErrReconvertNotSupported)
	assert.Empty(t, c.statuses)
}

func TestReconvert_running(t *testing.T) {
	s, c := newReconvertFixture("実行中", plateaucms.ConverterFlow)
	_, err := reconvert(context.Background(), s, &Config{}, ReconvertRequest{ProjectID: "p", Model: "plateau-bldg", ItemID: "main"})
	assert.ErrorIs(t, err, ErrReconvertRunning)
	assert.Empty(t, c.statuses)
}

func TestReconvert_invalidRequest(t *testing.T) {
	s, _ := newReconvertFixture("", plateaucms.ConverterFlow)
	_, err := reconvert(context.Background(), s, &Config{}, ReconvertRequest{ProjectID: "p", Model: "bldg", ItemID: "main"})
	assert.Error(t, err)
}
