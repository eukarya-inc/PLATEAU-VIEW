package cmsintegrationcommon

import (
	"context"
	"fmt"
	"strings"

	"github.com/oklog/ulid/v2"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/samber/lo"
)

func GenerateCMSID() string {
	return strings.ToLower(ulid.Make().String())
}

func GetMainItemWithMetadata(ctx context.Context, c cms.Interface, i *cms.Item) (_ *cms.Item, err error) {
	var mainItem, metadataItem *cms.Item

	if i.MetadataItemID == nil && i.OriginalItemID != nil {
		// item is metadata
		metadataItem = i
		mainItem, err = c.GetItem(ctx, *i.OriginalItemID, false)
		if err != nil || mainItem == nil {
			return nil, fmt.Errorf("failed to get main item: %w", err)
		}
	} else if i.OriginalItemID == nil && i.MetadataItemID != nil {
		// item is main
		mainItem = i
		metadataItem, err = c.GetItem(ctx, *i.MetadataItemID, false)
		if err != nil || metadataItem == nil {
			return nil, fmt.Errorf("failed to get metadata item: %w", err)
		}
	} else {
		return nil, fmt.Errorf("invalid item")
	}

	mainItem.MetadataItemID = lo.ToPtr(metadataItem.ID)
	mainItem.MetadataFields = metadataItem.Fields
	return mainItem, nil
}
