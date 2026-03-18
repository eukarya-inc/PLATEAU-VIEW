package cmsintflow

import (
	"path"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/datacatalogv3"
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

// Internal parses Flow outputs and extracts conversion results, dictionary, and QC results
func (r FlowResult) Internal() (res FlowInternalResult) {
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

		key := getOutputKey(base)
		if res.Conv == nil {
			res.Conv = map[string][]string{}
		}
		res.Conv[key] = append(res.Conv[key], output)
	}

	return
}

// getOutputKey extracts the key from an output filename using datacatalogv3.ParseAssetName
// The key format is "{type}_{name}_{format}" or "{type}_{format}" if name is empty
// e.g., "veg_PlantCover_3dtiles", "bldg_3dtiles", "fld_natl_river-name_3dtiles"
func getOutputKey(s string) string {
	name := datacatalogv3.ParseAssetName(fileName(s))
	if name == nil {
		// Fallback: return filename without extension
		return fileName(s)
	}

	ex := name.Ex
	switch {
	case ex.Fld != nil:
		// For flood data: "fld_{admin}_{river}_{format}"
		// e.g., "fld_natl_river-name_3dtiles"
		return "fld_" + ex.Fld.Admin + "_" + ex.Fld.River + "_" + ex.Fld.Format
	case ex.Normal != nil:
		// For normal feature types: "{type}_{name}_{format}" or "{type}_{format}" if name is empty
		// e.g., "veg_PlantCover_mvt", "veg_PlantCover_3dtiles", "bldg_3dtiles"
		if ex.Normal.Name != "" {
			return ex.Normal.Type + "_" + ex.Normal.Name + "_" + ex.Normal.Format
		}
		return ex.Normal.Type + "_" + ex.Normal.Format
	}

	// Fallback if extension couldn't be parsed
	return fileName(s)
}

func fileName(s string) string {
	return strings.TrimSuffix(path.Base(s), path.Ext(s))
}
