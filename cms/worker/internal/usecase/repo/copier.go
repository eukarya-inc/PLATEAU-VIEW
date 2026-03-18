package repo

import (
	"context"

	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/task"
	"go.mongodb.org/mongo-driver/bson"
)

type Copier interface {
	Copy(context.Context, bson.M, task.Changes) error
}
