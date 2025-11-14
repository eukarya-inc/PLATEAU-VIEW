package gql

import (
	"context"

	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/internal/adapter/gql/gqlmodel"
	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/internal/usecase/interfaces"
	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/id"
	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/schema"
	"github.com/reearth/reearthx/util"
	"github.com/samber/lo"
)

type SchemaLoader struct {
	usecase interfaces.Schema
}

func NewSchemaLoader(usecase interfaces.Schema) *SchemaLoader {
	return &SchemaLoader{usecase: usecase}
}

func (c *SchemaLoader) Fetch(ctx context.Context, ids []gqlmodel.ID) ([]*gqlmodel.Schema, []error) {
	sIDs, err := util.TryMap(ids, gqlmodel.ToID[id.Schema])
	if err != nil {
		return nil, []error{err}
	}

	res, err := c.usecase.FindByIDs(ctx, sIDs, getOperator(ctx))
	if err != nil {
		return nil, []error{err}
	}

	return lo.Map(sIDs, func(id schema.ID, _ int) *gqlmodel.Schema {
		s, ok := lo.Find(res, func(s *schema.Schema) bool {
			return s != nil && s.ID() == id
		})
		if !ok {
			return nil
		}
		return gqlmodel.ToSchema(s)
	}), nil
}
