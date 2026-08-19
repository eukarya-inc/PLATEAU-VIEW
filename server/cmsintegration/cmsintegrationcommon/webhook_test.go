package cmsintegrationcommon

import (
	"context"
	"testing"

	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/stretchr/testify/assert"
)

func TestPayloadModelKeyAndItemID(t *testing.T) {
	// non-item events (e.g. asset upload) carry no item data
	assetPayload := &cmswebhook.Payload{
		Type:      cmswebhook.EventAssetDecompress,
		AssetData: &cmswebhook.AssetData{ID: "a"},
	}
	assert.Empty(t, PayloadModelKey(assetPayload))
	assert.Empty(t, PayloadItemID(assetPayload))
	assert.Empty(t, PayloadModelKey(nil))
	assert.Empty(t, PayloadItemID(nil))

	itemPayload := &cmswebhook.Payload{
		Type: cmswebhook.EventItemCreate,
		ItemData: &cmswebhook.ItemData{
			Item:  &cms.Item{ID: "i"},
			Model: &cms.Model{Key: ModelPrefix + "bldg"},
		},
	}
	assert.Equal(t, ModelPrefix+"bldg", PayloadModelKey(itemPayload))
	assert.Equal(t, "i", PayloadItemID(itemPayload))
}

func TestValidatePayload(t *testing.T) {
	ctx := context.Background()

	assert.False(t, ValidatePayload(ctx, nil, "int"))

	// non-item event: must be rejected without panicking
	assert.False(t, ValidatePayload(ctx, &cmswebhook.Payload{
		Type:      cmswebhook.EventAssetDecompress,
		AssetData: &cmswebhook.AssetData{ID: "a"},
	}, "int"))

	// item event without item data
	assert.False(t, ValidatePayload(ctx, &cmswebhook.Payload{
		Type: cmswebhook.EventItemCreate,
	}, "int"))

	assert.True(t, ValidatePayload(ctx, &cmswebhook.Payload{
		Type: cmswebhook.EventItemCreate,
		ItemData: &cmswebhook.ItemData{
			Item:  &cms.Item{ID: "i"},
			Model: &cms.Model{Key: ModelPrefix + "bldg"},
		},
	}, "int"))
}
