package memory

import (
	"context"

	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/internal/usecase/repo"
)

type Lock struct{}

func NewLock() repo.Lock {
	return &Lock{}
}

func (r *Lock) Lock(_ context.Context, _ string) error {
	return nil
}

func (r *Lock) Unlock(_ context.Context, _ string) error {
	return nil
}
