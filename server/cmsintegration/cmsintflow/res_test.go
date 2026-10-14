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
