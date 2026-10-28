package plateauapi

import (
	"context"
	"fmt"
	"sync"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestRepos_ConcurrentProjects(t *testing.T) {
	r := NewRepos(func(ctx context.Context, project string) (*ReposUpdateResult, error) {
		return &ReposUpdateResult{
			Repo:     NewInMemoryRepo(&InMemoryRepoContext{}),
			Warnings: []string{project},
		}, nil
	})

	var wg sync.WaitGroup
	for i := 0; i < 20; i++ {
		project := fmt.Sprintf("p%d", i)
		wg.Add(1)
		go func() {
			defer wg.Done()
			_, err := r.Update(context.Background(), project)
			assert.NoError(t, err)
			_ = r.Repo(project)
			_ = r.Warnings(project)
			_ = r.Projects()
		}()
	}
	wg.Wait()

	assert.Len(t, r.Projects(), 20)
}
