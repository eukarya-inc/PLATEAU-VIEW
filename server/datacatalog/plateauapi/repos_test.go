package plateauapi

import (
	"context"
	"fmt"
	"sync"
	"testing"
)

// TestReposConcurrentAccess fails under -race if the shared maps are touched
// without holding Repos.mu: Update writes them for different projects in
// parallel while readers walk them.
func TestReposConcurrentAccess(t *testing.T) {
	r := NewRepos(func(_ context.Context, project string) (*ReposUpdateResult, error) {
		return &ReposUpdateResult{
			Repo:     NewInMemoryRepo(&InMemoryRepoContext{}),
			Warnings: []string{project},
		}, nil
	})

	ctx := context.Background()
	var wg sync.WaitGroup

	for i := range 20 {
		project := fmt.Sprintf("project%d", i)

		wg.Add(2)
		go func() {
			defer wg.Done()
			if _, err := r.Update(ctx, project); err != nil {
				t.Error(err)
			}
		}()
		go func() {
			defer wg.Done()
			r.Repo(project)
			r.Projects()
			r.Warnings(project)
			r.UpdatedAt(project)
		}()
	}

	wg.Wait()

	if got := len(r.Projects()); got != 20 {
		t.Errorf("got %d projects, want 20", got)
	}
}
