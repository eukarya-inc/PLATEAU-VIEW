package datacatalog

import (
	"context"
	"sync"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"golang.org/x/sync/singleflight"
)

// simpleDatasetsCacheEntries is the number of distinct results kept by the
// memo. Only the newest revision is normally read, but a couple of extra slots
// absorb requests that are still in flight across a repo update, and requests
// for a different project (pid) or a different visibility context.
const simpleDatasetsCacheEntries = 4

// simpleDatasetsCacheKey identifies a memoised FetchSimplePlateauDatasets
// result. The result is a pure function of the repo contents (captured by the
// repo revision), the host used to build absolute URLs, and the visibility
// flags carried by the request context — nothing else from the request leaks
// into it, so these three fields are a complete key.
type simpleDatasetsCacheKey struct {
	revision   string
	host       string
	visibility string
}

type simpleDatasetsCacheEntry struct {
	key   simpleDatasetsCacheKey
	value *SimpleDatasetsResponse
}

// simpleDatasetsCache memoises FetchSimplePlateauDatasets results. Building the
// response materialises the whole all-Japan catalog, which is far too expensive
// to repeat for every tileset.json / tilejson.json / CityGML redirect request
// that only needs a single entry out of it.
//
// Keying on the repo revision means a cached body is exactly the body the
// revision-derived ETag promises, so the memo cannot serve anything a
// conditional GET would not have served from the client cache anyway.
type simpleDatasetsCache struct {
	sf      singleflight.Group
	mu      sync.RWMutex
	entries []simpleDatasetsCacheEntry
}

func (c *simpleDatasetsCache) get(key simpleDatasetsCacheKey) (*SimpleDatasetsResponse, bool) {
	c.mu.RLock()
	defer c.mu.RUnlock()

	for _, e := range c.entries {
		if e.key == key {
			return e.value, true
		}
	}
	return nil, false
}

func (c *simpleDatasetsCache) set(key simpleDatasetsCacheKey, value *SimpleDatasetsResponse) {
	c.mu.Lock()
	defer c.mu.Unlock()

	for i, e := range c.entries {
		if e.key == key {
			c.entries[i].value = value
			return
		}
	}

	// Newest first, dropping the oldest entry beyond the bound.
	c.entries = append([]simpleDatasetsCacheEntry{{key: key, value: value}}, c.entries...)
	if len(c.entries) > simpleDatasetsCacheEntries {
		c.entries = c.entries[:simpleDatasetsCacheEntries]
	}
}

// fetchSimplePlateauDatasets returns the simple datasets response for the given
// repo, reusing a previously computed one whenever the repo revision, host and
// request visibility are unchanged. Concurrent misses are collapsed into a
// single computation via singleflight.
func (h *ReposHandler) fetchSimplePlateauDatasets(ctx context.Context, r plateauapi.Repo, host string) (*SimpleDatasetsResponse, error) {
	key := simpleDatasetsCacheKey{
		revision:   r.Revision(),
		host:       host,
		visibility: plateauapi.VisibilityKey(ctx),
	}

	// An empty revision means the repo cannot tell us when its data changed, so
	// there is nothing safe to key on.
	if key.revision == "" {
		return FetchSimplePlateauDatasets(ctx, r, host)
	}

	if cached, ok := h.simpleCache.get(key); ok {
		return cached, nil
	}

	sfKey := key.revision + "\x00" + key.host + "\x00" + key.visibility
	v, err, _ := h.simpleCache.sf.Do(sfKey, func() (any, error) {
		// Recheck under singleflight: a previous waiter may have populated the
		// cache while this goroutine was queued.
		if cached, ok := h.simpleCache.get(key); ok {
			return cached, nil
		}

		// The computation is read-only over already-loaded in-memory repos, so
		// the leader's context only affects how long it may run; unlike the CMS
		// fetches there is no upstream call to detach from cancellation.
		res, err := FetchSimplePlateauDatasets(ctx, r, host)
		if err != nil {
			return nil, err
		}

		h.simpleCache.set(key, res)
		return res, nil
	})
	if err != nil {
		return nil, err
	}
	return v.(*SimpleDatasetsResponse), nil
}
