package cmsintflow

import (
	"encoding/json"

	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationcommon"
)

type FlowRequest struct {
	TriggerID       string
	NotificationURL string
	AuthToken       string
	CityGMLURL      string
	ConvSettings    *cmsintegrationcommon.ConvSettings
	DryRun          bool
}

func (r FlowRequest) MarshalJSON() ([]byte, error) {
	if r.ConvSettings == nil {
		r.ConvSettings = &cmsintegrationcommon.ConvSettings{}
	}

	w := map[string]any{
		"cityGmlPath":    r.CityGMLURL,
		"targetPackages": []string{r.ConvSettings.FeatureType},
		"codelists":      r.ConvSettings.CodeLists,
		"schemas":        r.ConvSettings.Schemas,
	}

	if r.ConvSettings.PRCS != "" {
		w["prcs"] = r.ConvSettings.PRCS
	}
	if r.ConvSettings.ObjectLists != "" {
		w["objectLists"] = r.ConvSettings.ObjectLists
	}

	m := map[string]any{
		"notificationURL": r.NotificationURL,
		"authToken":       r.AuthToken,
		"with":            w,
	}

	return json.Marshal(m)
}

type FlowRequestResult map[string]any
