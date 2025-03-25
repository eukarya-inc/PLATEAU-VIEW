package cmsintmaxlod

import (
	"net/http"
	"strings"

	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationcommon"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/log"
)

type Config struct {
	GCPProject       string
	GCPRegion        string
	CMSBaseURL       string
	CMSToken         string
	CMSSystemProject string
	CMSIntegration   string
	WorkerImage      string
}

func WebhookHandler(conf Config) (cmswebhook.Handler, error) {
	s, err := NewServices(conf)
	if err != nil {
		return nil, err
	}

	return func(req *http.Request, w *cmswebhook.Payload) error {
		ctx := req.Context()
		ctx = log.WithPrefixMessage(ctx, "cmsintegrationmaxlod: ")

		log.Debugfc(ctx, "incoming: %+v", w)

		if !cmsintegrationcommon.ValidatePayload(ctx, w, conf.CMSIntegration) {
			return nil
		}

		modelName := strings.TrimPrefix(w.ItemData.Model.Key, cmsintegrationcommon.ModelPrefix)
		if err := extractMaxLOD(ctx, s, w); err != nil {
			log.Errorfc(ctx, "failed to process event: %v", err)
		}

		log.Debugfc(ctx, "done: %s", modelName)
		return nil
	}, nil
}
