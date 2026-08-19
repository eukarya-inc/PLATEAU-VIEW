package datacatalog

import (
	"context"
	"sync"
	"sync/atomic"
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/datacatalogv3"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// countingRepo counts how many times the catalog was actually walked, and lets
// the test control the revision the repo reports.
type countingRepo struct {
	*plateauapi.InMemoryRepo
	revision string
	calls    atomic.Int32
}

func newCountingRepo(revision string) *countingRepo {
	return &countingRepo{
		InMemoryRepo: plateauapi.NewInMemoryRepo(&plateauapi.InMemoryRepoContext{Name: "test"}),
		revision:     revision,
	}
}

func (r *countingRepo) Revision() string {
	return r.revision
}

func (r *countingRepo) Datasets(ctx context.Context, input *plateauapi.DatasetsInput) ([]plateauapi.Dataset, error) {
	r.calls.Add(1)
	return r.InMemoryRepo.Datasets(ctx, input)
}

func TestReposHandler_FetchSimplePlateauDatasets_Memoized(t *testing.T) {
	ctx := context.Background()
	h := &ReposHandler{host: "https://example.com"}
	r := newCountingRepo("rev1")

	first, err := h.fetchSimplePlateauDatasets(ctx, r, h.host)
	require.NoError(t, err)
	require.NotNil(t, first)
	assert.Equal(t, int32(1), r.calls.Load())

	// Repeated requests at the same revision reuse the computed value.
	for range 5 {
		got, err := h.fetchSimplePlateauDatasets(ctx, r, h.host)
		require.NoError(t, err)
		assert.Same(t, first, got)
	}
	assert.Equal(t, int32(1), r.calls.Load())

	// A different host is a different response, so it must not be shared.
	_, err = h.fetchSimplePlateauDatasets(ctx, r, "https://other.example.com")
	require.NoError(t, err)
	assert.Equal(t, int32(2), r.calls.Load())

	// A new revision invalidates the memo.
	r.revision = "rev2"
	second, err := h.fetchSimplePlateauDatasets(ctx, r, h.host)
	require.NoError(t, err)
	assert.NotSame(t, first, second)
	assert.Equal(t, int32(3), r.calls.Load())
}

// A request that can see admin-only entries must never read (or poison) the
// value computed for a public request, even at the same revision.
func TestReposHandler_FetchSimplePlateauDatasets_VisibilityIsolation(t *testing.T) {
	h := &ReposHandler{host: "https://example.com"}
	r := newCountingRepo("rev1")

	public := context.Background()
	admin := datacatalogv3.AdminContext(context.Background(), true, true, true)

	assert.NotEqual(t, plateauapi.VisibilityKey(public), plateauapi.VisibilityKey(admin))

	_, err := h.fetchSimplePlateauDatasets(public, r, h.host)
	require.NoError(t, err)
	assert.Equal(t, int32(1), r.calls.Load())

	_, err = h.fetchSimplePlateauDatasets(admin, r, h.host)
	require.NoError(t, err)
	assert.Equal(t, int32(2), r.calls.Load())

	// ...and each of them is cached separately afterwards.
	_, err = h.fetchSimplePlateauDatasets(public, r, h.host)
	require.NoError(t, err)
	_, err = h.fetchSimplePlateauDatasets(admin, r, h.host)
	require.NoError(t, err)
	assert.Equal(t, int32(2), r.calls.Load())
}

// Concurrent cold misses must collapse into a single computation.
func TestReposHandler_FetchSimplePlateauDatasets_Singleflight(t *testing.T) {
	ctx := context.Background()
	h := &ReposHandler{host: "https://example.com"}
	r := newCountingRepo("rev1")

	const workers = 20
	errs := make([]error, workers)
	var wg sync.WaitGroup
	for i := range workers {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			_, err := h.fetchSimplePlateauDatasets(ctx, r, h.host)
			errs[i] = err
		}(i)
	}
	wg.Wait()

	for _, err := range errs {
		assert.NoError(t, err)
	}
	assert.Equal(t, int32(1), r.calls.Load(), "singleflight should collapse concurrent misses")
}

// An unknown revision cannot be keyed on safely, so such repos are never cached.
func TestReposHandler_FetchSimplePlateauDatasets_NoRevision(t *testing.T) {
	ctx := context.Background()
	h := &ReposHandler{host: "https://example.com"}
	r := newCountingRepo("")

	for range 3 {
		_, err := h.fetchSimplePlateauDatasets(ctx, r, h.host)
		require.NoError(t, err)
	}
	assert.Equal(t, int32(3), r.calls.Load())
}

func TestSimpleDatasetsCache_Bounded(t *testing.T) {
	c := &simpleDatasetsCache{}
	keys := make([]simpleDatasetsCacheKey, 0, simpleDatasetsCacheEntries+1)
	for i := range simpleDatasetsCacheEntries + 1 {
		k := simpleDatasetsCacheKey{revision: string(rune('a' + i))}
		keys = append(keys, k)
		c.set(k, &SimpleDatasetsResponse{})
	}

	// The oldest entry is evicted; every newer one is still there.
	_, ok := c.get(keys[0])
	assert.False(t, ok)
	for _, k := range keys[1:] {
		_, ok := c.get(k)
		assert.True(t, ok)
	}
}
