package repo

import (
	"context"

	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/asset"
)

type AssetUpload interface {
	Save(ctx context.Context, upload *asset.Upload) error
	FindByID(ctx context.Context, uuid string) (*asset.Upload, error)
}
