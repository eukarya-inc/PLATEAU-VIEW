package plateaucms

import (
	"context"
	"net/http"
	"net/url"
	"testing"

	"github.com/jarcoal/httpmock"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestMinorVersionsFromMax(t *testing.T) {
	assert.Equal(
		t,
		[]string{"3.0", "3.1", "3.2", "3.3", "3.4", "3.5"},
		minorVersionsFromMax(3, 5),
	)
	assert.Equal(
		t,
		[]string{"4.0", "4.1"},
		minorVersionsFromMax(4, 1),
	)
	assert.Equal(
		t,
		[]string{"4.0"},
		minorVersionsFromMax(4, 0),
	)
}

func TestCMS_PlateauSpecs(t *testing.T) {
	httpmock.Activate()
	defer httpmock.Deactivate()
	mockCMSPlateauSpec(t)

	cms := &CMS{
		cmsbase:       testCMSHost,
		cmsSysProject: tokenProject,
		cmsMain:       lo.Must(cms.New(testCMSHost, testCMSToken)),
	}

	expected := []PlateauSpec{
		{
			ID:              "1",
			MajorVersion:    4,
			Year:            2024,
			MaxMinorVersion: 1,
			FMEURL:          "https://example.com/v4",
		},
		{
			ID:              "2",
			MajorVersion:    3,
			Year:            2023,
			MaxMinorVersion: 5,
			FMEURL:          "https://example.com/v3",
		},
	}

	specs, err := cms.PlateauSpecs(context.Background())
	assert.NoError(t, err)
	assert.Equal(t, expected, specs)
}

func mockCMSPlateauSpec(t *testing.T) {
	t.Helper()

	httpmock.RegisterResponder(
		"GET",
		lo.Must(url.JoinPath(testCMSHost, "api", "projects", tokenProject, "models", plateauSpecModel, "items")),
		httpmock.NewJsonResponderOrPanic(http.StatusOK, cms.Items{
			PerPage:    1,
			Page:       1,
			TotalCount: 1,
			Items: []cms.Item{
				{
					ID: "1",
					Fields: []*cms.Field{
						{Key: "major_version", Value: 4},
						{Key: "year", Value: 2024},
						{Key: "max_minor_version", Value: 1},
						{Key: "fme_url", Value: "https://example.com/v4"},
					},
				},
				{
					ID: "2",
					Fields: []*cms.Field{
						{Key: "major_version", Value: 3},
						{Key: "year", Value: 2023},
						{Key: "max_minor_version", Value: 5},
						{Key: "fme_url", Value: "https://example.com/v3"},
					},
				},
			},
		}),
	)
}

func TestPlateauSpec_IsFMEEnabled(t *testing.T) {
	tests := []struct {
		name      string
		converter string
		want      bool
	}{
		{
			name:      "empty converter: returns true (default to FME)",
			converter: "",
			want:      true,
		},
		{
			name:      "fme converter: returns true",
			converter: ConverterFME,
			want:      true,
		},
		{
			name:      "flow converter: returns false",
			converter: ConverterFlow,
			want:      false,
		},
		{
			name:      "fme_flow converter: returns true",
			converter: ConverterFMEFlow,
			want:      true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			s := PlateauSpec{
				Converter: tt.converter,
			}
			got := s.IsFMEEnabled()
			assert.Equal(t, tt.want, got)
		})
	}
}

func TestPlateauSpec_IsFlowEnabled(t *testing.T) {
	tests := []struct {
		name      string
		converter string
		want      bool
	}{
		{
			name:      "empty converter: returns false",
			converter: "",
			want:      false,
		},
		{
			name:      "fme converter: returns false",
			converter: ConverterFME,
			want:      false,
		},
		{
			name:      "flow converter: returns true",
			converter: ConverterFlow,
			want:      true,
		},
		{
			name:      "fme_flow converter: returns true",
			converter: ConverterFMEFlow,
			want:      true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			s := PlateauSpec{
				Converter: tt.converter,
			}
			got := s.IsFlowEnabled()
			assert.Equal(t, tt.want, got)
		})
	}
}

func TestPlateauSpec_ShouldUseFlow(t *testing.T) {
	tests := []struct {
		name         string
		converter    string
		flowTriggers []FlowTrigger
		featureType  string
		want         bool
	}{
		{
			name:        "flow mode: always returns true",
			converter:   ConverterFlow,
			featureType: "bldg",
			want:        true,
		},
		{
			name:        "fme mode: always returns false",
			converter:   ConverterFME,
			featureType: "bldg",
			want:        false,
		},
		{
			name:        "empty converter: returns false",
			converter:   "",
			featureType: "bldg",
			want:        false,
		},
		{
			name:        "fme_flow mode: no triggers returns false",
			converter:   ConverterFMEFlow,
			featureType: "bldg",
			want:        false,
		},
		{
			name:      "fme_flow mode: trigger exists and flow not disabled returns true",
			converter: ConverterFMEFlow,
			flowTriggers: []FlowTrigger{
				{FeatureType: "bldg", FlowDisabled: false},
				{FeatureType: "tran", FlowDisabled: false},
			},
			featureType: "bldg",
			want:        true,
		},
		{
			name:      "fme_flow mode: trigger exists but flow disabled returns false",
			converter: ConverterFMEFlow,
			flowTriggers: []FlowTrigger{
				{FeatureType: "bldg", FlowDisabled: true},
				{FeatureType: "tran", FlowDisabled: false},
			},
			featureType: "bldg",
			want:        false,
		},
		{
			name:      "fme_flow mode: no trigger for feature type returns false",
			converter: ConverterFMEFlow,
			flowTriggers: []FlowTrigger{
				{FeatureType: "bldg", FlowDisabled: false},
				{FeatureType: "tran", FlowDisabled: false},
			},
			featureType: "fld",
			want:        false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			s := PlateauSpec{
				Converter:    tt.converter,
				FlowTriggers: tt.flowTriggers,
			}
			got := s.ShouldUseFlow(tt.featureType)
			assert.Equal(t, tt.want, got)
		})
	}
}

func TestPlateauSpec_GetFlowTrigger(t *testing.T) {
	spec := PlateauSpec{
		FlowTriggers: []FlowTrigger{
			{FeatureType: "bldg", FlowQCTrigger: "qc-bldg", FlowConvTrigger: "conv-bldg"},
			{FeatureType: "tran", FlowQCTrigger: "qc-tran", FlowConvTrigger: "conv-tran"},
		},
	}

	// Found
	trigger := spec.GetFlowTrigger("bldg")
	assert.NotNil(t, trigger)
	assert.Equal(t, "bldg", trigger.FeatureType)
	assert.Equal(t, "qc-bldg", trigger.FlowQCTrigger)
	assert.Equal(t, "conv-bldg", trigger.FlowConvTrigger)

	// Not found
	assert.Nil(t, spec.GetFlowTrigger("fld"))

	// Empty triggers
	emptySpec := PlateauSpec{}
	assert.Nil(t, emptySpec.GetFlowTrigger("bldg"))
}

func TestPlateauSpec_GetFlowQCTrigger(t *testing.T) {
	spec := PlateauSpec{
		FlowTriggers: []FlowTrigger{
			{FeatureType: "bldg", FlowQCTrigger: "qc-bldg", FlowConvTrigger: "conv-bldg"},
		},
	}

	assert.Equal(t, "qc-bldg", spec.GetFlowQCTrigger("bldg"))
	assert.Equal(t, "", spec.GetFlowQCTrigger("fld"))
}

func TestPlateauSpec_GetFlowConvTrigger(t *testing.T) {
	spec := PlateauSpec{
		FlowTriggers: []FlowTrigger{
			{FeatureType: "bldg", FlowQCTrigger: "qc-bldg", FlowConvTrigger: "conv-bldg"},
		},
	}

	assert.Equal(t, "conv-bldg", spec.GetFlowConvTrigger("bldg"))
	assert.Equal(t, "", spec.GetFlowConvTrigger("fld"))
}

func TestPlateauSpecList_FindByVersion(t *testing.T) {
	list := PlateauSpecList{
		{MajorVersion: 3, Year: 2023},
		{MajorVersion: 4, Year: 2024},
	}

	// Found
	spec := list.FindByVersion(4)
	assert.NotNil(t, spec)
	assert.Equal(t, 4, spec.MajorVersion)

	// Not found
	assert.Nil(t, list.FindByVersion(5))

	// Empty list
	emptyList := PlateauSpecList{}
	assert.Nil(t, emptyList.FindByVersion(4))
}

func TestPlateauSpecList_FindByYear(t *testing.T) {
	list := PlateauSpecList{
		{MajorVersion: 3, Year: 2023},
		{MajorVersion: 4, Year: 2024},
	}

	// Found
	spec := list.FindByYear(2024)
	assert.NotNil(t, spec)
	assert.Equal(t, 2024, spec.Year)

	// Not found
	assert.Nil(t, list.FindByYear(2025))

	// Empty list
	emptyList := PlateauSpecList{}
	assert.Nil(t, emptyList.FindByYear(2024))
}

func TestPlateauSpec_GetEffectiveFlowURL(t *testing.T) {
	tests := []struct {
		name       string
		specURL    string
		defaultURL string
		want       string
	}{
		{
			name:       "spec URL set: returns spec URL",
			specURL:    "https://spec.example.com",
			defaultURL: "https://default.example.com",
			want:       "https://spec.example.com",
		},
		{
			name:       "spec URL empty: returns default URL",
			specURL:    "",
			defaultURL: "https://default.example.com",
			want:       "https://default.example.com",
		},
		{
			name:       "both empty: returns empty",
			specURL:    "",
			defaultURL: "",
			want:       "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			s := PlateauSpec{FlowURL: tt.specURL}
			got := s.GetEffectiveFlowURL(tt.defaultURL)
			assert.Equal(t, tt.want, got)
		})
	}
}
