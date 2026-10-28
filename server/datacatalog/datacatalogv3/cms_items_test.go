package datacatalogv3

import (
	"context"
	"net/http"
	"testing"

	"github.com/jarcoal/httpmock"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestGetItemsAndConv_Errors(t *testing.T) {
	ctx := context.Background()
	httpmock.Activate()
	defer httpmock.DeactivateAndReset()

	c := lo.Must(cms.New("https://example.com", "token"))
	conv := func(i cms.Item) *string { return &i.ID }

	// the model exists but the items cannot be fetched
	httpmock.RegisterResponder("GET", "https://example.com/api/projects/prj/models/plateau-city/items",
		httpmock.NewStringResponder(http.StatusInternalServerError, "error"))
	httpmock.RegisterResponder("GET", "https://example.com/api/projects/prj/models/plateau-city",
		httpmock.NewJsonResponderOrPanic(http.StatusOK, cms.Model{ID: "city"}))

	var warnings []string
	addWarning := func(w string) { warnings = append(warnings, w) }

	res, err := getItemsAndConv(c, ctx, "prj", "plateau-city", addWarning, conv)
	assert.Error(t, err)
	assert.Nil(t, res)
	assert.Empty(t, warnings)

	// the model does not exist in the project
	httpmock.RegisterResponder("GET", "https://example.com/api/projects/prj/models/plateau-rfld/items",
		httpmock.NewStringResponder(http.StatusNotFound, "not found"))
	httpmock.RegisterResponder("GET", "https://example.com/api/projects/prj/models/plateau-rfld",
		httpmock.NewStringResponder(http.StatusNotFound, "not found"))

	res, err = getItemsAndConv(c, ctx, "prj", "plateau-rfld", addWarning, conv)
	assert.NoError(t, err)
	assert.Nil(t, res)
	assert.Len(t, warnings, 1)
}
