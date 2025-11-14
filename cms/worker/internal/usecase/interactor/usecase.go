package interactor

import (
	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/worker/internal/usecase/gateway"
	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/worker/internal/usecase/repo"
)

type Usecase struct {
	gateways *gateway.Container
	repos    *repo.Container
}

func NewUsecase(g *gateway.Container, r *repo.Container) *Usecase {
	return &Usecase{
		gateways: g,
		repos:    r,
	}
}
