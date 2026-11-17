package plateauapi

import (
	"context"
	"fmt"
	"sync"
	"sync/atomic"
	"testing"
	"time"

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

func TestRepos_ConcurrentUpdatesOfSameProject(t *testing.T) {
	var calls atomic.Int32
	r := NewRepos(func(ctx context.Context, project string) (*ReposUpdateResult, error) {
		calls.Add(1)
		time.Sleep(10 * time.Millisecond)
		return &ReposUpdateResult{Repo: NewInMemoryRepo(&InMemoryRepoContext{})}, nil
	})

	done := make(chan struct{})
	go func() {
		var wg sync.WaitGroup
		for i := 0; i < 50; i++ {
			wg.Add(1)
			go func() {
				defer wg.Done()
				_, err := r.Update(context.Background(), "p")
				assert.NoError(t, err)
			}()
		}
		wg.Wait()
		close(done)
	}()

	select {
	case <-done:
	case <-time.After(5 * time.Second):
		t.Fatal("concurrent updates of the same project did not finish")
	}
	assert.Equal(t, int32(1), calls.Load())
}
