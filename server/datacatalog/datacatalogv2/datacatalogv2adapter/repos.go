package datacatalogv2adapter

import (
	"context"
	"fmt"
	"time"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/reearth/reearthx/log"
	"github.com/reearth/reearthx/util"
)

type Repos struct {
	fetchers *util.SyncMap[string, *Fetcher]
	*plateauapi.Repos
}

func NewRepos() *Repos {
	r := &Repos{
		fetchers: util.NewSyncMap[string, *Fetcher](),
	}
	r.Repos = plateauapi.NewRepos(r.update)
	return r
}

// Prepare registers the fetcher for a project but doesn't fetch data.
// Actual fetching happens lazily when GetOrFetch is called.
func (r *Repos) Prepare(_ context.Context, f *Fetcher) error {
	project := f.Project()
	if _, ok := r.fetchers.Load(project); ok {
		return nil
	}

	r.setCMS(f)
	return nil
}

// GetOrFetch returns the repo for a project, fetching from CMS if not cached.
func (r *Repos) GetOrFetch(ctx context.Context, project string) *plateauapi.RepoWrapper {
	repo := r.Repo(project)
	if repo != nil {
		return repo
	}

	// If fetcher is not registered, return nil
	if _, ok := r.fetchers.Load(project); !ok {
		return nil
	}

	// Fetch and cache
	if _, err := r.Update(ctx, project); err != nil {
		return nil
	}

	return r.Repo(project)
}

func (r *Repos) update(ctx context.Context, project string) (*plateauapi.ReposUpdateResult, error) {
	fetcher, ok := r.fetchers.Load(project)
	if !ok {
		return nil, fmt.Errorf("fetcher is not initialized for %s", project)
	}

	updated := r.UpdatedAt(project)
	var updatedStr string
	if !updated.IsZero() {
		updatedStr = updated.Format(time.RFC3339)
	}
	log.Debugfc(ctx, "datacatalogv2: updating repo %s: last_update=%s", project, updatedStr)

	repo, err := fetchAndCreateCache(ctx, fetcher)
	if err != nil {
		return nil, err
	}

	log.Debugfc(ctx, "datacatalogv2: updated repo %s", project)

	return &plateauapi.ReposUpdateResult{
		Repo: repo,
	}, nil
}

func (r *Repos) setCMS(f *Fetcher) {
	r.fetchers.Store(f.Project(), f)
}
