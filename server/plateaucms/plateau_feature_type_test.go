package plateaucms

import (
	"context"
	"net/http"
	"net/url"
	"sync"
	"testing"
	"time"

	"github.com/jarcoal/httpmock"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestPlateauFeatureTypeFrom(t *testing.T) {
	i := &cms.Item{
		Fields: []*cms.Field{
			{Key: "code", Value: "bldg"},
			{Key: "name", Value: "Building"},
			{Key: "qc", Value: true},
			{Key: "conv", Value: true},
		},
	}

	res := PlateauFeatureTypeFrom(i)
	assert.Equal(t, "bldg", res.Code)
	assert.Equal(t, "Building", res.Name)
	assert.True(t, res.QC)
	assert.True(t, res.Conv)

	assert.Nil(t, PlateauFeatureTypeFrom(nil))
}

// Note: Flow trigger ID tests have been moved to plateau_spec_test.go
// as Flow settings are now managed in PlateauSpec.FlowTriggers instead of PlateauFeatureType

func mockFeatureTypesCMS(model string, items []cms.Item) string {
	u := lo.Must(url.JoinPath(testCMSHost, "api", "projects", tokenProject, "models", model, "items"))
	httpmock.RegisterResponder(
		"GET",
		u,
		httpmock.NewJsonResponderOrPanic(http.StatusOK, cms.Items{
			PerPage:    100,
			Page:       1,
			TotalCount: len(items),
			Items:      items,
		}),
	)
	return "GET " + u
}

// The public CityGML files API needs the feature types on every request, so
// repeated calls within the cache TTL must hit the upstream CMS at most once,
// and concurrent misses must be deduplicated via singleflight.
func TestHandler_PlateauFeatureTypes_Cache(t *testing.T) {
	httpmock.Activate()
	defer httpmock.Deactivate()
	key := mockFeatureTypesCMS(plateauFeatureTypesModel, []cms.Item{
		{ID: "1", Fields: []*cms.Field{{Key: "code", Value: "bldg"}, {Key: "name", Value: "建築物モデル"}}},
	})

	h := newHandler()
	h.featureTypeCacheTTL = time.Minute

	ctx := context.Background()

	got, err := h.PlateauFeatureTypes(ctx)
	require.NoError(t, err)
	assert.Equal(t, []string{"bldg"}, got.Codes())
	baseline := httpmock.GetCallCountInfo()[key]
	assert.Positive(t, baseline)

	// Subsequent calls within the TTL must not hit upstream again.
	for range 5 {
		got, err := h.PlateauFeatureTypes(ctx)
		require.NoError(t, err)
		assert.Equal(t, []string{"bldg"}, got.Codes())
	}
	assert.Equal(t, baseline, httpmock.GetCallCountInfo()[key])

	// Concurrent misses on a fresh handler should be deduplicated by singleflight.
	// Errors are collected per goroutine and asserted on the main goroutine
	// (testify's assertions are not safe to call concurrently).
	h2 := newHandler()
	h2.featureTypeCacheTTL = time.Minute
	const workers = 10
	errs := make([]error, workers)
	var wg sync.WaitGroup
	for i := range workers {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			_, err := h2.PlateauFeatureTypes(ctx)
			errs[i] = err
		}(i)
	}
	wg.Wait()
	for _, err := range errs {
		assert.NoError(t, err)
	}
	after := httpmock.GetCallCountInfo()[key]
	assert.LessOrEqual(t, after-baseline, baseline, "singleflight should collapse concurrent misses to a single upstream fetch")

	// An expired cache refetches.
	h.featureTypeFetched = time.Now().Add(-2 * time.Minute)
	_, err = h.PlateauFeatureTypes(ctx)
	require.NoError(t, err)
	assert.Greater(t, httpmock.GetCallCountInfo()[key], after)
}

func TestHandler_DatasetTypes_Cache(t *testing.T) {
	httpmock.Activate()
	defer httpmock.Deactivate()
	key := mockFeatureTypesCMS(datasetTypesModel, []cms.Item{
		{ID: "1", Fields: []*cms.Field{{Key: "code", Value: "shelter"}, {Key: "name", Value: "避難施設"}, {Key: "category", Value: DatasetCategoryRelated}}},
	})

	h := newHandler()
	h.featureTypeCacheTTL = time.Minute

	ctx := context.Background()

	got, err := h.DatasetTypes(ctx)
	require.NoError(t, err)
	assert.Equal(t, []string{"shelter"}, got.Codes(DatasetCategoryRelated))
	baseline := httpmock.GetCallCountInfo()[key]
	assert.Positive(t, baseline)

	for range 5 {
		_, err := h.DatasetTypes(ctx)
		require.NoError(t, err)
	}
	assert.Equal(t, baseline, httpmock.GetCallCountInfo()[key])
}

// Without a TTL (the zero value used by tests and by hand-built instances)
// nothing is cached.
func TestHandler_PlateauFeatureTypes_NoCache(t *testing.T) {
	httpmock.Activate()
	defer httpmock.Deactivate()
	key := mockFeatureTypesCMS(plateauFeatureTypesModel, []cms.Item{
		{ID: "1", Fields: []*cms.Field{{Key: "code", Value: "bldg"}}},
	})

	h := newHandler()
	ctx := context.Background()

	_, err := h.PlateauFeatureTypes(ctx)
	require.NoError(t, err)
	baseline := httpmock.GetCallCountInfo()[key]
	_, err = h.PlateauFeatureTypes(ctx)
	require.NoError(t, err)
	assert.Greater(t, httpmock.GetCallCountInfo()[key], baseline)
}

// A clone must not inherit the cache state of the instance it was cloned from.
func TestHandler_Clone_FeatureTypeCacheNotCopied(t *testing.T) {
	h := newHandler()
	h.featureTypeCacheTTL = time.Minute
	h.storePlateauFeatureTypes(PlateauFeatureTypeList{{Code: "bldg"}})
	h.storeDatasetTypes(DatasetTypeList{{Code: "shelter"}})

	c := h.Clone()
	assert.Equal(t, time.Minute, c.featureTypeCacheTTL)
	_, ok := c.cachedPlateauFeatureTypes()
	assert.False(t, ok)
	_, ok = c.cachedDatasetTypes()
	assert.False(t, ok)
}
