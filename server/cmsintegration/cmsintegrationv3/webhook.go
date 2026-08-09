package cmsintegrationv3

import (
	"net/http"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/log"
)

func WebhookHandler(conf Config) (cmswebhook.Handler, error) {
	s, err := NewServices(conf)
	if err != nil {
		return nil, err
	}

	return func(req *http.Request, w *cmswebhook.Payload) error {
		ctx := req.Context()
		ctx = log.WithPrefixMessage(ctx, "cmsintegrationv3 webhook: ")

		// ValidatePayload is what rejects deliveries with a nil ItemData /
		// Model / Item, so the pre-validation log must not dereference them —
		// otherwise a webhook whose shape is exactly what the validator exists
		// to filter panics before it can be filtered.
		log.Infofc(ctx, "incoming: type=%s, project=%s", w.Type, w.ProjectID())
		log.Debugfc(ctx, "incoming payload: %+v", w)

		if !cmsintegrationcommon.ValidatePayload(ctx, w, conf.CMSIntegration) {
			log.Infofc(ctx, "validation failed, skipping")
			return nil
		}

		log.Infofc(ctx, "validated: model=%s, item=%s",
			w.ItemData.Model.Key, w.ItemData.Item.ID)

		modelName := strings.TrimPrefix(w.ItemData.Model.Key, cmsintegrationcommon.ModelPrefix)
		log.Infofc(ctx, "processing: model=%s, item=%s", modelName, w.ItemData.Item.ID)

		err := sendRequestToFME(ctx, s, &conf, w)
		if err != nil {
			log.Errorfc(ctx, "failed to process event: model=%s, item=%s, err=%v", modelName, w.ItemData.Item.ID, err)
		}

		log.Infofc(ctx, "done: model=%s, item=%s", modelName, w.ItemData.Item.ID)
		return nil
	}, nil
}
