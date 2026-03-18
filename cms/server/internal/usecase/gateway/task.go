package gateway

import (
	"context"

	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/task"
)

type TaskRunner interface {
	Run(context.Context, task.Payload) error
	Retry(context.Context, string) error
}
