package repo

import (
	"context"

	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/event"
	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/id"
)

type Event interface {
	FindByID(context.Context, id.EventID) (*event.Event[any], error)
	Save(context.Context, *event.Event[any]) error
	SaveAll(context.Context, event.List) error
}
