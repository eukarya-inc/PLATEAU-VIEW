package repo

import (
	"context"

	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/id"
	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/item/view"
)

type View interface {
	Filtered(ProjectFilter) View
	FindByIDs(context.Context, id.ViewIDList) (view.List, error)
	FindByModel(context.Context, id.ModelID) (view.List, error)
	FindByID(context.Context, id.ViewID) (*view.View, error)
	Save(context.Context, *view.View) error
	SaveAll(context.Context, view.List) error
	Remove(context.Context, id.ViewID) error
}
