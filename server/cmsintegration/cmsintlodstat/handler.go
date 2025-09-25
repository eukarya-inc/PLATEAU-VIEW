package cmsintlodstat

import (
	"context"
	"fmt"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/gcptaskrunner"
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
		_ = s.CMS.CommentToItem(ctx, w.ItemData.Item.ID, fmt.Sprintf("LOD抽出をスキップしました: 無効なフィーチャータイプ (%s)", modelName))
		return nil
	}

	if !ft.LODStat {
		log.Debugfc(ctx, "maxlod: lodStat is false: %s", modelName)
		_ = s.CMS.CommentToItem(ctx, w.ItemData.Item.ID, fmt.Sprintf("LOD抽出をスキップしました: フィーチャータイプ %s でLOD抽出が無効になっています", modelName))
		return nil
	}

	mainItem, err := s.GetMainItemWithMetadata(ctx, w.ItemData.Item)
	if err != nil {
		return fmt.Errorf("maxlod: failed to get main item: %w", err)
	} else if mainItem.MetadataItemID == nil {
		_ = s.CMS.CommentToItem(ctx, w.ItemData.Item.ID, "LOD抽出をスキップしました: メタデータアイテムが存在しません")
		return fmt.Errorf("maxlod: main item has no metadata")
	}

	if tag := mainItem.MetadataFieldByKey("maxlod_status").GetValue().Tag(); tag == nil {
		log.Debugfc(ctx, "maxlod_status metadata is missing")
		_ = s.CMS.CommentToItem(ctx, mainItem.ID, "LOD抽出をスキップしました: maxlod_statusフィールドが見つかりません")
		return nil
	} else if tag.Name != "" && tag.Name != "未実行" {
		log.Debugfc(ctx, "already running")
		// 実行中の場合はコメントしない（頻繁になるため）
		return nil
	}

	city := lo.FromPtr(mainItem.FieldByKey("city").GetValue().String())
	if city == "" {
		log.Debugfc(ctx, "city not found")
		_ = s.CMS.CommentToItem(ctx, mainItem.ID, "LOD抽出をスキップしました: cityフィールドが空です")
		return nil
	}

	asset := *mainItem.FieldByKey("citygml").GetValue().String()
	if asset == "" {
		log.Debugfc(ctx, "citygml not updated")
		_ = s.CMS.CommentToItem(ctx, mainItem.ID, "LOD抽出をスキップしました: citygmlフィールドが空です")
		return nil
	}

	assetURL := ""
	if a, err := s.CMS.Asset(ctx, asset); err == nil {
		assetURL = a.URL
	} else {
		log.Debugfc(ctx, "asset not found: %v", err)
		_ = s.CMS.CommentToItem(ctx, mainItem.ID, fmt.Sprintf("LOD抽出をスキップしました: アセットが見つかりません (%s)", asset))
		return nil
	}

	log.Debugfc(ctx, "run")

	if err := s.TaskRunner.Run(ctx, gcptaskrunner.Task{
		Args: []string{
			"lodstat",
			"-src=" + assetURL,
			"-project=" + w.ProjectID(),
			"-item=" + mainItem.ID,
			"-feature=" + ft.Code,
		},
	}, &gcptaskrunner.Config{
		Tags: []string{"lodstat"},
	}); err != nil {
		return fmt.Errorf("maxlod: failed to run task: %w", err)
	}

	if _, err := s.CMS.UpdateItem(ctx, *mainItem.MetadataItemID, nil, []*cms.Field{
		{
			ID:    "maxlod_status",
			Type:  "tag",
			Value: "実行中",
		},
	}); err != nil {
		log.Errorfc(ctx, "cmsintegrationv3: maxlod: failed to update item: %v", err)
	}

	_ = s.CMS.CommentToItem(ctx, mainItem.ID, "CityGMLが変更されたためLOD抽出を開始しました。")

	log.Debugfc(ctx, "done")
	return nil
}
