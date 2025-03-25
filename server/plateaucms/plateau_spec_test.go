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
