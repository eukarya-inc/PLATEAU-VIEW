package cmsintegrationcommon

import (
	"context"
	"strings"

	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/log"
)

func ValidatePayload(ctx context.Context, w *cmswebhook.Payload, cmsintegration string) bool {
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
