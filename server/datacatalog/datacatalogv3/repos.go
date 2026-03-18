package datacatalogv3

import (
	"context"
	"encoding/json"
	"fmt"
	"sort"
	"time"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/log"
	"github.com/reearth/reearthx/util"
)

func AdminContext(ctx context.Context, bypassAdminRemoval, includeBeta, includeAlpha bool) context.Context {
	if bypassAdminRemoval {
		ctx = plateauapi.BypassAdminRemoval(ctx, true)
	}
	var stages []string
	if includeBeta {
		stages = append(stages, string(stageBeta))
	}
	if includeAlpha {
		stages = append(stages, string(stageAlpha))
	}
	if len(stages) > 0 {
		ctx = plateauapi.AllowAdminStages(ctx, stages)
	}
	return ctx
}

type Repos struct {
	pcms   *plateaucms.CMS
	cms    *util.SyncMap[string, *CMS]
	cache  bool
	debug  bool
	host   string
	writer RepoWriter
	*plateauapi.Repos
}

func NewRepos(pcms *plateaucms.CMS) *Repos {
	r := &Repos{
		pcms:   pcms,
		cms:    util.NewSyncMap[string, *CMS](),
		writer: NewFileRepoWriter("cache"), // Default to file writer
	}
	r.Repos = plateauapi.NewRepos(r.update)
	return r
}

func (r *Repos) EnableCache(cache bool) {
	r.cache = cache
}

func (r *Repos) EnableDebug(debug bool) {
	r.debug = debug
}

func (r *Repos) SetHost(host string) {
	r.host = host
}

func (r *Repos) SetWriter(writer RepoWriter) {
	r.writer = writer
}

func (r *Repos) Prepare(ctx context.Context, project string, year int, plateau bool, cms cms.Interface) error {
	// Skip if already prepared (cms registered)
	if _, ok := r.cms.Load(project); ok {
		return nil
	}

	// Skip if repo already exists (e.g., loaded from cache)
	if r.Repo(project) != nil {
		return nil
	}

	r.setCMS(project, year, plateau, cms)
	_, err := r.Update(ctx, project)
	return err
}

func (r *Repos) update(ctx context.Context, project string) (*plateauapi.ReposUpdateResult, error) {
	cms, ok := r.cms.Load(project)
	if !ok {
		return nil, fmt.Errorf("cms is not initialized for %s", project)
	}

	{
		updated := r.UpdatedAt(project)
		updatedStr := ""
		if !updated.IsZero() {
			updatedStr = fmt.Sprintf(": last_update=%s", updated.Format(time.RFC3339))
		}
		log.Debugfc(ctx, "datacatalogv3: updating repo %s%s", project, updatedStr)
	}

	t := time.Now()

	data, err := cms.GetAll(ctx, r.host)
	if err != nil {
		return nil, err
	}

	log.Debugfc(ctx, "datacatalogv3: updating repo %s (fetch completed in %.2fs)", project, time.Since(t).Seconds())

	c, warning := data.Into()
	sort.Strings(warning)

	var repo *plateauapi.InMemoryRepo
	if c != nil {
		repo = plateauapi.NewInMemoryRepo(c)
	}

	log.Debugfc(ctx, "datacatalogv3: updated repo %s: %.2fs", project, time.Since(t).Seconds())

	if r.debug && r.writer != nil {
		dumpRepo(ctx, repo, c, warning, project, r.writer)
	}

	return &plateauapi.ReposUpdateResult{
		Repo:     repo,
		Warnings: warning,
	}, nil
}

func (r *Repos) setCMS(project string, year int, plateau bool, cms cms.Interface) {
	c := NewCMS(CMSOpts{
		CMS:     cms,
		PCMS:    r.pcms,
		Year:    year,
		Plateau: plateau,
		Project: project,
		Cache:   r.cache,
	})
	r.cms.Store(project, c)
}

func dumpRepo(ctx context.Context, _ *plateauapi.InMemoryRepo, c *plateauapi.InMemoryRepoContext, warning []string, project string, writer RepoWriter) {
	// Get writer for JSON data
	f, err := writer.GetWriter(project)
	if err != nil {
		log.Errorfc(ctx, "datacatalogv3: failed to get writer for repo_%s.json: %v", project, err)
		return
	}
	defer func() {
		_ = f.Close()
	}()

	// Write JSON data
	d := json.NewEncoder(f)
	d.SetIndent("", "  ")
	if err := d.Encode(c); err != nil {
		log.Errorfc(ctx, "datacatalogv3: failed to write repo_%s.json: %v", project, err)
	}

	// Write warnings if any
	if len(warning) > 0 {
		wf, err := writer.GetWarningWriter(project)
		if err != nil {
			log.Errorfc(ctx, "datacatalogv3: failed to get warning writer for repo_%s_warnings.txt: %v", project, err)
			return
		}
		defer func() {
			_ = wf.Close()
		}()

		for _, w := range warning {
			if _, err := wf.Write([]byte(w + "\n")); err != nil {
				log.Errorfc(ctx, "datacatalogv3: failed to write repo_%s_warnings.txt: %v", project, err)
			}
		}
	}

	log.Debugfc(ctx, "datacatalogv3: wrote repo_%s.json", project)
}
