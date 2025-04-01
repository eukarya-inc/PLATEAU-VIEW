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
	MaxLOD   string
	QCResult string
	QCOK     bool
}

func (r FlowResult) IsSucceeded() bool {
	return r.Status == "succeeded"
}

func (r FlowResult) IsFailed() bool {
	return r.Status == "failed"
}

func (r FlowResult) Internal() (res FlowInternalResult) {
	for _, output := range r.Outputs {
		base := path.Base(output)

		switch {
		case strings.HasSuffix(base, "dic.json"):
			res.Dic = output
			continue
		case strings.HasSuffix(base, "maxlod.csv") || strings.HasSuffix(base, "maxLod.csv"):
			res.MaxLOD = output
			continue
		case strings.HasSuffix(base, "qc_result.zip"):
			res.QCResult = output
			continue
		case strings.HasSuffix(base, "qc_result_succeeded"):
			res.QCOK = true
			continue
		}

		if path.Ext(base) != ".zip" {
			continue
		}

		key := getOutputKey(base)
		if res.Conv == nil {
			res.Conv = map[string][]string{}
		}
		res.Conv[key] = append(res.Conv[key], output)
	}

	return
}

var reDigits = regexp.MustCompile(`^\d+_(.*)$`)

func getOutputKey(s string) string {
	k := reDigits.ReplaceAllString(fileName(s), "$1")
	k = strings.TrimSuffix(k, "_no_texture")
	k = strings.TrimSuffix(k, "_l1")
	k = strings.TrimSuffix(k, "_l2")
	k = strings.TrimSuffix(k, "_lod0")
	k = strings.TrimSuffix(k, "_lod1")
	k = strings.TrimSuffix(k, "_lod2")
	k = strings.TrimSuffix(k, "_lod3")
	k = strings.TrimSuffix(k, "_lod4")
	return k
}

func fileName(s string) string {
	return strings.TrimSuffix(path.Base(s), path.Ext(s))
}
