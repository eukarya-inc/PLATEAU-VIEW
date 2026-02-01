package cmsintflow

import (
	"path"
	"regexp"
	"strings"
)

type FlowResult struct {
	ID           string   `json:"-"`
	RunID        string   `json:"runId"`
	TriggerID    string   `json:"triggerId"`
	DeploymentID string   `json:"deploymentId"`
	Status       string   `json:"status"`
	Logs         []string `json:"logs"`
	Outputs      []string `json:"outputs"`
}

type FlowInternalResult struct {
	Conv     map[string][]string
	Dic      string
	QCResult string
	QCOK     bool
}

func (r FlowResult) IsSucceeded() bool {
	return r.Status == "succeeded"
}

func (r FlowResult) IsFailed() bool {
	return r.Status == "failed"
}

func (r FlowResult) IDsMessage() string {
	var ids []string
	if r.RunID != "" {
		ids = append(ids, "RunID: "+r.RunID)
	}
	if r.DeploymentID != "" {
		ids = append(ids, "DeploymentID: "+r.DeploymentID)
	}
	if r.TriggerID != "" {
		ids = append(ids, "TriggerID: "+r.TriggerID)
	}
	if len(ids) == 0 {
		return ""
	}
	return "（" + strings.Join(ids, ", ") + "）"
}

// InternalWithFeatureType parses Flow outputs with feature type context for proper key extraction
func (r FlowResult) InternalWithFeatureType(featureTypeCode string, useGroups bool) (res FlowInternalResult) {
	for _, output := range r.Outputs {
		base := path.Base(output)

		switch {
		case strings.HasSuffix(base, "dic.json") || base == "dictionary.json":
			res.Dic = output
			continue
		case strings.HasSuffix(base, "qc_result.zip"):
			res.QCResult = output
			continue
		case strings.HasSuffix(base, "qc_result_succeeded") || strings.HasSuffix(base, "qc_result_ok"):
			res.QCOK = true
			continue
		}

		if path.Ext(base) != ".zip" {
			continue
		}

		key := getOutputKey(base, featureTypeCode, useGroups)
		if res.Conv == nil {
			res.Conv = map[string][]string{}
		}
		res.Conv[key] = append(res.Conv[key], output)
	}

	return
}

// Internal parses Flow outputs (legacy method without feature type context)
func (r FlowResult) Internal() (res FlowInternalResult) {
	return r.InternalWithFeatureType("", false)
}

var reDigits = regexp.MustCompile(`^\d+_(.*)$`)

func getOutputKey(s string, featureTypeCode string, useGroups bool) string {
	k := reDigits.ReplaceAllString(fileName(s), "$1")

	// For feature types that use groups (UseGroups=true), extract the feature type key
	// e.g., "uwajima-shi_city_2025_citygml_1_op_urf_UseDistrict_mvt_lod1" -> "urf_UseDistrict"
	if useGroups && featureTypeCode != "" {
		pattern := "_op_" + featureTypeCode + "_"
		if idx := strings.Index(k, pattern); idx != -1 {
			// Extract from "_op_" onward, skip "_op_" itself
			k = k[idx+4:]
			return trimOutputKeySuffixes(k)
		}
	}

	// For bldg and other feature types (default behavior)
	return trimOutputKeySuffixes(k)
}

func trimOutputKeySuffixes(s string) string {
	// Loop to handle combined suffixes like "_lod2_no_texture" or "_mvt_lod1"
	for {
		prev := s
		s = strings.TrimSuffix(s, "_lod0")
		s = strings.TrimSuffix(s, "_lod1")
		s = strings.TrimSuffix(s, "_lod2")
		s = strings.TrimSuffix(s, "_lod3")
		s = strings.TrimSuffix(s, "_lod4")
		s = strings.TrimSuffix(s, "_lod")
		s = strings.TrimSuffix(s, "_mvt")
		s = strings.TrimSuffix(s, "_no_texture")
		s = strings.TrimSuffix(s, "_l1")
		s = strings.TrimSuffix(s, "_l2")
		if s == prev {
			break
		}
	}
	return s
}

func fileName(s string) string {
	return strings.TrimSuffix(path.Base(s), path.Ext(s))
}
