package cmsintegrationcommon

import (
	"context"
	"strings"

	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/log"
)

// PayloadModelKey returns the model key of the payload.
// It returns an empty string when the payload has no item data,
// which is the case for non-item events such as asset uploads.
func PayloadModelKey(w *cmswebhook.Payload) string {
	if w == nil || w.ItemData == nil || w.ItemData.Model == nil {
		return ""
	}
	return w.ItemData.Model.Key
}

// PayloadItemID returns the item ID of the payload.
// It returns an empty string when the payload has no item data,
// which is the case for non-item events such as asset uploads.
func PayloadItemID(w *cmswebhook.Payload) string {
	if w == nil || w.ItemData == nil || w.ItemData.Item == nil {
		return ""
	}
	return w.ItemData.Item.ID
}

func ValidatePayload(ctx context.Context, w *cmswebhook.Payload, cmsintegration string) bool {
	if w == nil {
		log.Debugfc(ctx, "invalid event: no payload")
		return false
	}

	if !w.Operator.IsUser() && w.Operator.IsIntegrationBy(cmsintegration) {
		log.Debugfc(ctx, "invalid event operator: %+v", w.Operator)
		return false
	}

	if w.Type != cmswebhook.EventItemCreate && w.Type != cmswebhook.EventItemUpdate {
		log.Debugfc(ctx, "invalid event type: %s", w.Type)
		return false
	}

	if w.ItemData == nil || w.ItemData.Item == nil || w.ItemData.Model == nil {
		log.Debugfc(ctx, "invalid event data: %+v", w.Data)
		return false
	}

	if !strings.HasPrefix(w.ItemData.Model.Key, ModelPrefix) {
		log.Debugfc(ctx, "invalid model id: %s, key: %s", w.ItemData.Item.ModelID, w.ItemData.Model.Key)
		return false
	}

	return true
}
