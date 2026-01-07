package cmsintflow

import (
	"net/http"
	"net/url"
	"path"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/k0kubun/pp/v3"
	"github.com/labstack/echo/v4"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/log"
)

func init() {
	pp.ColoringEnabled = false
}

const handlerPath = "/notify_flow"

func WebhookHandler(conf Config) (cmswebhook.Handler, error) {
	s, err := NewServices(conf)
	if err != nil {
		return nil, err
	}

	return func(req *http.Request, w *cmswebhook.Payload) error {
		ctx := req.Context()
		ctx = log.WithPrefixMessage(ctx, "cmsintflow webhook: ")

		log.Infofc(ctx, "incoming: type=%s, project=%s, model=%s, item=%s",
			w.Type, w.ProjectID(), w.ItemData.Model.Key, w.ItemData.Item.ID)
		log.Debugfc(ctx, "incoming payload: %+v", w)
		if !cmsintegrationcommon.ValidatePayload(ctx, w, conf.CMSIntegration) {
			log.Infofc(ctx, "validation failed, skipping")
			return nil
		}

		// if event type is "item.create" and payload is metadata, skip it
		if w.Type == cmswebhook.EventItemCreate && (w.ItemData.Item.OriginalItemID != nil || w.ItemData.Item.IsMetadata) {
			log.Infofc(ctx, "skip: metadata item creation event")
			return nil
		}

		log.Debugfc(ctx, "getting main item with metadata")
		mainItem, err := s.GetMainItemWithMetadata(ctx, w.ItemData.Item)
		if err != nil {
			log.Errorfc(ctx, "failed to get main item with metadata: %v", err)
			return err
		}
		log.Infofc(ctx, "main item: id=%s", mainItem.ID)

		md, _, err := s.PCMS.Metadata(ctx, w.ProjectID(), false, false)
		if err != nil {
			log.Errorfc(ctx, "failed to get metadata: %v", err)
			return nil
		}
		log.Debugfc(ctx, "metadata: %s", pp.Sprint(md))
		if !md.IsFlowEnabled() {
			log.Debugfc(ctx, "flow is disabled")
			return nil
		}

		modelName := strings.TrimPrefix(w.ItemData.Model.Key, cmsintegrationcommon.ModelPrefix)
		if md.Converter == plateaucms.ConverterFMEFlow && modelName != "flow" { // plateau-flow model for testing Flow
			log.Infofc(ctx, "skip: model %s is not flow model (converter=%s)", modelName, md.Converter)
			return nil
		}

		log.Debugfc(ctx, "getting feature types")
		featureTypes, err := s.PCMS.PlateauFeatureTypes(ctx)
		if err != nil {
			log.Errorfc(ctx, "failed to get feature types: %v", err)
			return nil
		}
		log.Debugfc(ctx, "feature types: count=%d", len(featureTypes))

		log.Infofc(ctx, "sending request to flow: project=%s, model=%s, item=%s", w.ProjectID(), modelName, mainItem.ID)
		if err := sendRequestToFlow(ctx, s, &conf, w.ProjectID(), modelName, mainItem, featureTypes, ""); err != nil {
			log.Errorfc(ctx, "failed to trigger flow: %v", err)
			return nil
		}

		log.Infofc(ctx, "done: project=%s, model=%s, item=%s", w.ProjectID(), modelName, mainItem.ID)
		return nil
	}, nil
}

func Handler(conf Config, g *echo.Group) error {
	s, err := NewServices(conf)
	if err != nil {
		return err
	}

	g.POST(path.Join(handlerPath, ":id"), func(c echo.Context) error {
		ctx := c.Request().Context()
		ctx = log.WithPrefixMessage(ctx, "cmsintflow notify: ")
		id := c.Param("id")

		log.Infofc(ctx, "incoming: id=%s", id)

		if id == "" {
			log.Warnfc(ctx, "empty id")
			return c.JSON(http.StatusBadRequest, "empty id")
		}

		var f FlowResult
		if err := c.Bind(&f); err != nil {
			log.Warnfc(ctx, "invalid payload: %v", err)
			return c.JSON(http.StatusBadRequest, "invalid payload")
		}

		f.ID = id
		log.Infofc(ctx, "received: status=%s, runID=%s, triggerID=%s", f.Status, f.RunID, f.TriggerID)
		log.Debugfc(ctx, "received payload: %#v", f)

		if err := receiveResultFromFlow(ctx, s, &conf, f); err != nil {
			log.Errorfc(ctx, "failed to receive result from flow: %v", err)
			return c.JSON(http.StatusInternalServerError, "failed to receive result from flow")
		}

		log.Infofc(ctx, "done: id=%s", id)
		return nil
	})

	return nil
}

func resultURL(conf *Config, sig string) string {
	u, _ := url.JoinPath(conf.Host, handlerPath, sig)
	return u
}
