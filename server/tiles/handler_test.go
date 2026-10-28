package tiles

import (
	"context"
	"errors"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func TestHandler_GetTiles_FailureBackoff(t *testing.T) {
	now := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	calls := 0
	var fetchErr error
	stale := Tiles{"a": {}}

	h := &Handler{
		tiles:     stale,
		fetchedAt: now,
		now:       func() time.Time { return now },
		fetch: func(ctx context.Context) (Tiles, error) {
			calls++
			if fetchErr != nil {
				return nil, fetchErr
			}
			return Tiles{"b": {}}, nil
		},
	}
	ctx := context.Background()

	// the cache expires while the CMS is down: the stale cache is served
	now = now.Add(tilesTTL + time.Second)
	fetchErr = errors.New("cms down")
	assert.Equal(t, stale, h.getTiles(ctx))
	assert.Equal(t, 1, calls)

	// within the backoff, the CMS is not queried again
	now = now.Add(tilesFailureBackoff / 2)
	assert.Equal(t, stale, h.getTiles(ctx))
	assert.Equal(t, 1, calls)

	// after the backoff, the refresh is retried and succeeds
	now = now.Add(tilesFailureBackoff)
	fetchErr = nil
	assert.Equal(t, Tiles{"b": {}}, h.getTiles(ctx))
	assert.Equal(t, 2, calls)
}
