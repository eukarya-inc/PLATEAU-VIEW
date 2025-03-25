package cmsintmaxlod

import (
	"context"
	"fmt"
	"strings"

	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/gcptaskrunner"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/log"
	"github.com/samber/lo"
)

func extractMaxLOD(ctx context.Context, s *Services, w *cmswebhook.Payload) error {
	if s.TaskRunner == nil {
		return nil
	}

	// if event type is "item.create" and payload is metadata, skip it
	if w.Type == cmswebhook.EventItemCreate && w.ItemData.Item.OriginalItemID != nil ||
		w.ItemData == nil || w.ItemData.Item == nil || w.ItemData.Model == nil {
		return nil
	}

	// feature types
	modelName := strings.TrimPrefix(w.ItemData.Model.Key, cmsintegrationcommon.ModelPrefix)
	featureTypes, err := s.PCMS.PlateauFeatureTypes(ctx)
	if err != nil {
		return fmt.Errorf("maxlod: failed to get feature types: %w", err)
	}

	ft, ok := featureTypes.GetByCode(modelName)
	if !ok {
		log.Debugfc(ctx, "invalid feature type: %s", modelName)
		return nil
	}

	if ft.Conv {
		log.Debugfc(ctx, "no need to extract maxlod: %s", modelName)
		return nil
	}

	mainItem, err := s.GetMainItemWithMetadata(ctx, w.ItemData.Item)
	if err != nil {
		return fmt.Errorf("maxlod: failed to get main item: %w", err)
	}

	if tag := mainItem.MetadataFieldByKey("maxlod_status").GetValue().Tag(); tag != nil && tag.Name != "" && tag.Name != "未実行" {
		log.Debugfc(ctx, "already running")
		return nil
	}

	city := lo.FromPtr(mainItem.FieldByKey("city").GetValue().String())
	if city == "" {
		log.Debugfc(ctx, "city not found")
		return nil
	}

	asset := *mainItem.FieldByKey("citygml").GetValue().String()
	if asset == "" {
		log.Debugfc(ctx, "citygml not updated")
		return nil
	}

	log.Debugfc(ctx, "run")

	if err := s.TaskRunner.Run(ctx, gcptaskrunner.Task{
		Args: []string{
			"extract-maxlod",
			"--city=" + city,
			"--project=" + w.ProjectID(),
			"--ftypes=dem",
			"--overwrite",
			"--wetrun",
		},
	}, &gcptaskrunner.Config{
		Tags: []string{"maxlod"},
	}); err != nil {
		return fmt.Errorf("maxlod: failed to run task: %w", err)
	}

	if _, err := s.CMS.UpdateItem(ctx, mainItem.ID, nil, []*cms.Field{
		{
			ID:    "maxlod_status",
			Type:  "tag",
			Value: "実行中",
		},
	}); err != nil {
		log.Errorfc(ctx, "cmsintegrationv3: maxlod: failed to update item: %v", err)
	}

	_ = s.CMS.CommentToItem(ctx, mainItem.ID, "CityGMLが変更されたため最大LOD抽出を開始しました。")

	log.Debugfc(ctx, "done")
	return nil
}
