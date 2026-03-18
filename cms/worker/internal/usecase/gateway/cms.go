package gateway

import (
	"context"

	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/worker/pkg/asset"
)

type CMS interface {
	NotifyAssetDecompressed(ctx context.Context, assetID string, status *asset.ArchiveExtractionStatus) error
}
