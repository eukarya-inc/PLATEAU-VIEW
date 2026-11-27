package cmsintflow

import (
	"context"
	"errors"
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/stretchr/testify/assert"
)

type failingPCMS struct{ PCMS }

func (failingPCMS) PlateauFeatureTypes(context.Context) (plateaucms.PlateauFeatureTypeList, error) {
	return nil, errors.New("cms down")
}

// A transient CMS error while ingesting a result must mark the item failed and
// surface an error, not ack the callback and leave the item "running".
func TestReceiveResultFromFlow_IngestErrorMarksItemFailed(t *testing.T) {
	c := &cmsintegrationcommon.CMSMock{}
	var updates [][]*cms.Field
	var comments []string
	c.MockUpdateItem = func(ctx context.Context, id string, fields, metadataFields []*cms.Field) (*cms.Item, error) {
		assert.Equal(t, "item", id)
		updates = append(updates, metadataFields)
		return nil, nil
	}
	c.MockCommentToItem = func(ctx context.Context, id, comment string) error {
		comments = append(comments, comment)
		return nil
	}
	s := &Services{CMS: c, PCMS: failingPCMS{}}
	conf := &Config{Secret: "secret"}
	id := ID{ItemID: "item", ProjectID: "prj", FeatureType: "bldg", Type: cmsintegrationcommon.ReqTypeQC}

	err := receiveResultFromFlow(context.Background(), s, conf, FlowResult{ID: id.Sign(conf.Secret), Status: "succeeded"})

	assert.ErrorContains(t, err, "cms down")
	assert.Len(t, comments, 1)
	assert.Contains(t, comments[0], "cms down")
	// error status, then the run ID cleared
	assert.Len(t, updates, 2)
	assert.Equal(t, "flow_run_id", updates[1][0].Key)
}

// A conversion started for a CRS must not replace the assets of another CRS.
func TestReceiveResultFromFlow_UnexpectedCRS(t *testing.T) {
	run := func(t *testing.T, outputName string) (dataUpdated bool, comments []string) {
		c := &cmsintegrationcommon.CMSMock{}
		c.MockGetItem = func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
			return &cms.Item{ID: id, Fields: []*cms.Field{{Key: "city", Value: "city"}, {Key: "data", Value: []any{"old"}}}}, nil
		}
		c.MockUploadAsset = func(ctx context.Context, projectID, url string) (string, error) { return "new", nil }
		c.MockAsset = func(ctx context.Context, id string) (*cms.Asset, error) {
			return &cms.Asset{ID: id, URL: "https://example.com/13999_x_op_bldg_3dtiles_lod1.zip"}, nil
		}
		c.MockUpdateItem = func(ctx context.Context, id string, fields, metadataFields []*cms.Field) (*cms.Item, error) {
			for _, f := range fields {
				if f.Key == "data" {
					dataUpdated = true
				}
			}
			return &cms.Item{}, nil
		}
		c.MockCommentToItem = func(ctx context.Context, id, comment string) error {
			comments = append(comments, comment)
			return nil
		}
		s := &Services{CMS: c, PCMS: reconvertPCMS{
			featureTypes: plateaucms.PlateauFeatureTypeList{{Code: "bldg", Conv: true}},
		}}
		conf := &Config{Secret: "secret"}
		id := ID{ItemID: "item", ProjectID: "prj", FeatureType: "bldg", Type: cmsintegrationcommon.ReqTypeConv, CRS: cmsintegrationcommon.CRSJGD2024}

		err := receiveResultFromFlow(context.Background(), s, conf, FlowResult{
			ID: id.Sign(conf.Secret), Status: "succeeded",
			Outputs: []string{"https://example.com/" + outputName},
		})
		assert.NoError(t, err)
		return
	}

	t.Run("jgd2011 output is rejected", func(t *testing.T) {
		updated, comments := run(t, "13999_x_op_bldg_3dtiles_lod1.zip")
		assert.False(t, updated)
		assert.Len(t, comments, 1)
		assert.Contains(t, comments[0], "測地系")
	})

	t.Run("jgd2024 output is merged", func(t *testing.T) {
		updated, _ := run(t, "13999_x_op_bldg_3dtiles_lod1_jgd2024.zip")
		assert.True(t, updated)
	})
}
