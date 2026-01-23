package plateaucms

import (
	"context"
	"errors"
	"fmt"

	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/rerror"
)

// Converter constants for FME/Flow settings
const (
	ConverterFME     = "fme"
	ConverterFlow    = "flow"
	ConverterFMEFlow = "fme_flow"
)

type SpecStore interface {
	PlateauSpecs(context.Context) ([]PlateauSpec, error)
}

var _ SpecStore = &CMS{}

// FlowTrigger holds Flow trigger IDs for a specific feature type
type FlowTrigger struct {
	FeatureType     string `json:"feature_type" cms:"feature_type,text"`
	FlowQCTrigger   string `json:"flow_qc_trigger" cms:"flow_qc_trigger,text"`
	FlowConvTrigger string `json:"flow_conv_trigger" cms:"flow_conv_trigger,text"`
	FlowDisabled    bool   `json:"flow_disabled" cms:"flow_disabled,bool"`
}

type PlateauSpec struct {
	ID              string `json:"id" cms:"id,text"`
	MajorVersion    int    `json:"major_version" cms:"major_version,integer"`
	Year            int    `json:"year" cms:"year,integer"`
	MaxMinorVersion int    `json:"max_minor_version" cms:"max_minor_version,integer"`
	FMEURL          string `json:"fme_url" cms:"fme_url,text"`
	AttrList        string `json:"attr_list" cms:"-"`

	// FME/Flow settings
	FlowURL      string        `json:"flow_url" cms:"flow_url,text"`
	Converter    string        `json:"converter" cms:"converter,select"`
	FlowTriggers []FlowTrigger `json:"flow_triggers" cms:"flow_triggers,group"`
}

func (s PlateauSpec) MinorVersions() []string {
	return minorVersionsFromMax(s.MajorVersion, s.MaxMinorVersion)
}

func minorVersionsFromMax(major, max int) []string {
	res := make([]string, 0, max)
	for i := 0; i <= max; i++ {
		res = append(res, fmt.Sprintf("%d.%d", major, i))
	}
	return res
}

// IsFMEEnabled returns whether FME is enabled for this spec
func (s PlateauSpec) IsFMEEnabled() bool {
	return IsFMEEnabledConverter(s.Converter)
}

// IsFlowEnabled returns whether Flow is enabled for this spec
func (s PlateauSpec) IsFlowEnabled() bool {
	return IsFlowEnabledConverter(s.Converter)
}

// IsFMEEnabledConverter returns whether FME is enabled for the given converter value
func IsFMEEnabledConverter(converter string) bool {
	return converter == "" || converter == ConverterFME || converter == ConverterFMEFlow
}

// IsFlowEnabledConverter returns whether Flow is enabled for the given converter value
func IsFlowEnabledConverter(converter string) bool {
	return converter == ConverterFlow || converter == ConverterFMEFlow
}

// ShouldUseFlow returns whether Flow should be used for the given feature type.
// In fme_flow mode, it checks if the feature type has a FlowTrigger with FlowDisabled=false.
func (s PlateauSpec) ShouldUseFlow(featureType string) bool {
	switch s.Converter {
	case ConverterFlow:
		return true // Flow-only mode: all feature types use Flow
	case ConverterFMEFlow:
		// Check if there's a FlowTrigger for this feature type with FlowDisabled=false
		trigger := s.GetFlowTrigger(featureType)
		if trigger == nil {
			return false // No trigger configured for this feature type
		}
		return !trigger.FlowDisabled
	default:
		return false // FME-only or unset
	}
}

// GetFlowTrigger returns the FlowTrigger for the given feature type
func (s PlateauSpec) GetFlowTrigger(featureType string) *FlowTrigger {
	for i := range s.FlowTriggers {
		if s.FlowTriggers[i].FeatureType == featureType {
			return &s.FlowTriggers[i]
		}
	}
	return nil
}

// GetFlowQCTrigger returns the Flow QC trigger ID for the given feature type
func (s PlateauSpec) GetFlowQCTrigger(featureType string) string {
	if t := s.GetFlowTrigger(featureType); t != nil {
		return t.FlowQCTrigger
	}
	return ""
}

// GetFlowConvTrigger returns the Flow conversion trigger ID for the given feature type
func (s PlateauSpec) GetFlowConvTrigger(featureType string) string {
	if t := s.GetFlowTrigger(featureType); t != nil {
		return t.FlowConvTrigger
	}
	return ""
}

// GetEffectiveFlowURL returns the Flow URL, using the spec's FlowURL if set, otherwise the default
func (s PlateauSpec) GetEffectiveFlowURL(defaultURL string) string {
	if s.FlowURL != "" {
		return s.FlowURL
	}
	return defaultURL
}

func (h *CMS) PlateauSpecs(ctx context.Context) ([]PlateauSpec, error) {
	if h.cmsSysProject == "" {
		return nil, rerror.ErrNotFound
	}

	items, err := h.cmsMain.GetItemsByKeyInParallel(ctx, h.cmsSysProject, plateauSpecModel, true, 100)
	if err != nil || items == nil {
		if errors.Is(err, cms.ErrNotFound) || items == nil {
			return nil, rerror.ErrNotFound
		}
		return nil, rerror.ErrInternalBy(fmt.Errorf("plateaucms: failed to get plateau-spec: %w", err))
	}

	all := make([]PlateauSpec, 0, len(items.Items))
	for _, item := range items.Items {
		m := PlateauSpec{}
		item.Unmarshal(&m)

		m.AttrList = valueToAssetURL(item.FieldByKey("attr_list").GetValue())

		all = append(all, m)
	}

	return all, nil
}

// PlateauSpecList is a list of PlateauSpec
type PlateauSpecList []PlateauSpec

// FindByVersion finds a PlateauSpec by major version
func (l PlateauSpecList) FindByVersion(majorVersion int) *PlateauSpec {
	for i := range l {
		if l[i].MajorVersion == majorVersion {
			return &l[i]
		}
	}
	return nil
}

// FindByYear finds a PlateauSpec by year
func (l PlateauSpecList) FindByYear(year int) *PlateauSpec {
	for i := range l {
		if l[i].Year == year {
			return &l[i]
		}
	}
	return nil
}
