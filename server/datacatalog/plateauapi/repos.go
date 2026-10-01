package plateauapi

import (
	"context"
	"fmt"
	"slices"
	"sort"
	"sync"
	"time"

	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/util"
	"github.com/samber/lo"
)

const cacheUpdateDuration = 10 * time.Second

type ReposUpdater = func(ctx context.Context, project string) (*ReposUpdateResult, error)

type ReposUpdateResult struct {
	Repo     Repo
	Warnings []string
}

type Repos struct {
	updater ReposUpdater
	locks   util.LockMap[string]
	// mu guards repos, warnings and updatedAt. locks only serializes updates
	// per project, so it cannot protect these shared maps across projects.
	mu        sync.RWMutex
	repos     map[string]*RepoWrapper
	warnings  map[string][]string
	updatedAt map[string]time.Time
	now       func() time.Time
}

func NewRepos(u ReposUpdater) *Repos {
	return &Repos{
		updater:   u,
		locks:     util.LockMap[string]{},
		repos:     map[string]*RepoWrapper{},
		warnings:  map[string][]string{},
		updatedAt: map[string]time.Time{},
	}
}

func (r *Repos) Prepare(ctx context.Context, project string, year int, cms cms.Interface) error {
	_, err := r.Update(ctx, project)
	return err
}

func (r *Repos) Repo(project string) *RepoWrapper {
	r.mu.RLock()
	defer r.mu.RUnlock()

	return r.repos[project]
}

func (r *Repos) Projects() []string {
	r.mu.RLock()
	keys := lo.Keys(r.repos)
	r.mu.RUnlock()

	sort.Strings(keys)
	return keys
}

func (r *Repos) UpdateAll(ctx context.Context) error {
	projects := r.Projects()
	for _, project := range projects {
		if _, err := r.Update(ctx, project); err != nil {
			return fmt.Errorf("failed to update project %s: %w", project, err)
		}
	}
	return nil
}

// Update updates the project's repo if it's not updated recently. If false is returned, it means the repo is not updated.
func (r *Repos) Update(ctx context.Context, project string) (bool, error) {
	r.locks.Lock(project)
	defer r.locks.Unlock(project)

	// avoid too frequent updates
	updated := r.UpdatedAt(project)
	since := r.getNow().Sub(updated)
	if !updated.IsZero() && since < cacheUpdateDuration {
		return false, nil
	}

	// update
	ur, err := r.updater(ctx, project)
	if err != nil {
		return false, fmt.Errorf("failed to update project %s: %w", project, err)
	}

	u := false
	if ur != nil {
		u = ur.Repo != nil
		r.store(project, ur.Repo, ur.Warnings, u)
	}

	return u, nil
}

// store installs or refreshes the wrapper for a project and records its warnings.
// A nil repo leaves the current wrapper untouched; touchUpdatedAt controls whether
// the project counts as freshly updated.
func (r *Repos) store(project string, repo Repo, warnings []string, touchUpdatedAt bool) {
	r.mu.Lock()
	defer r.mu.Unlock()

	if repo != nil {
		repoWrapper := r.repos[project]
		if repoWrapper == nil {
			repoWrapper = NewRepoWrapper(repo, nil)
			repoWrapper.SetName(project)
			r.repos[project] = repoWrapper
		} else {
			repoWrapper.SetRepo(repo)
		}
	}

	r.warnings[project] = warnings

	if touchUpdatedAt {
		r.updatedAt[project] = r.getNow()
	}
}

func (r *Repos) Warnings(project string) []string {
	r.mu.RLock()
	defer r.mu.RUnlock()

	if r.updatedAt[project].IsZero() {
		return []string{"project is not initialized"}
	}
	return slices.Clone(r.warnings[project])
}

func (r *Repos) UpdatedAt(project string) time.Time {
	r.mu.RLock()
	defer r.mu.RUnlock()

	return r.updatedAt[project]
}

func (r *Repos) getNow() time.Time {
	if r.now != nil {
		return r.now()
	}
	return time.Now()
}

// SetRepo directly sets a repo for a project without going through the updater.
// This is useful for loading cached data.
func (r *Repos) SetRepo(project string, repo Repo, warnings []string) {
	r.locks.Lock(project)
	defer r.locks.Unlock(project)

	r.store(project, repo, warnings, true)
}
