package cmsintflow

import (
	"context"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/log"
	"github.com/samber/lo"
)

// shouldCancelFlow checks if the webhook payload indicates a status change that should cancel the Flow job.
// Returns true only for item.update events on metadata items where conv_status or qc_status changed to non-running.
func shouldCancelFlow(w *cmswebhook.Payload) bool {
	// Only item.update events
	if w.Type != cmswebhook.EventItemUpdate {
		return false
	}

	// Only metadata items (those with OriginalItemID)
	if w.ItemData.Item.OriginalItemID == nil {
		return false
	}

	// Check if conv_status or qc_status changed to non-running
	return hasStatusChangeToNonRunning(w, "conv_status") ||
		hasStatusChangeToNonRunning(w, "qc_status")
}

// hasStatusChangeToNonRunning checks if a status field changed to a non-running status.
// The field must have changed and the current value must not be "running" (実行中).
func hasStatusChangeToNonRunning(w *cmswebhook.Payload, fieldKey string) bool {
	// w.ItemData.Item is a metadata item, so we need to use FieldByKey instead of MetadataFieldByKey
	field := w.ItemData.Item.FieldByKey(fieldKey)
	if field == nil {
		return false
	}

	change, ok := lo.Find(w.ItemData.Changes, func(c cms.FieldChange) bool {
		return c.ID == field.ID
	})
	if !ok {
		return false
	}

	// Check the current value - if it's not "running", this is a cancellation trigger
	currValue := change.GetCurrentValue()
	if currValue == nil {
		return false
	}

	// Check if it's a tag
	currTag := currValue.Tag()
	if currTag != nil {
		return currTag.Name != string(cmsintegrationcommon.ConvertionStatusRunning)
	}

	// Check if it's a string (some CMSes may return string instead of tag)
	currStr := currValue.String()
	if currStr != nil {
		return *currStr != string(cmsintegrationcommon.ConvertionStatusRunning)
	}

	return false
}

// handleFlowCancellation cancels the Flow job for the given item.
func handleFlowCancellation(ctx context.Context, s *Services, conf *Config, w *cmswebhook.Payload) error {
	log.Infofc(ctx, "handling flow cancellation: itemID=%s", w.ItemData.Item.ID)

	mainItem, err := s.GetMainItemWithMetadata(ctx, w.ItemData.Item)
	if err != nil {
		log.Errorfc(ctx, "failed to get main item with metadata: %v", err)
		return err
	}

	featureItem := cmsintegrationcommon.FeatureItemFrom(mainItem)
	if featureItem == nil || featureItem.FlowRunID == "" {
		log.Infofc(ctx, "no flow run id, skipping cancellation: itemID=%s", mainItem.ID)
		return nil
	}

	log.Infofc(ctx, "cancelling flow job: itemID=%s, runID=%s", mainItem.ID, featureItem.FlowRunID)

	// Get plateau specs to determine the correct Flow URL
	specs, err := s.PCMS.PlateauSpecs(ctx)
	if err != nil {
		log.Warnfc(ctx, "failed to get plateau specs, using default flow url: %v", err)
	}

	// Determine Flow base URL
	flowBaseURL := conf.FlowBaseURL
	specv := featureItem.SpecMajorVersionInt()
	if specv > 0 && specs != nil {
		spec := plateaucms.PlateauSpecList(specs).FindByVersion(specv)
		if spec != nil {
			flowBaseURL = spec.GetEffectiveFlowURL(conf.FlowBaseURL)
		}
	}

	// Cancel the Flow job
	if err := s.Flow.Cancel(ctx, flowBaseURL, featureItem.FlowRunID); err != nil {
		log.Warnfc(ctx, "failed to cancel flow job: %v", err)
		// Don't return error - cancellation failure should not block user operations
	} else {
		log.Infofc(ctx, "flow job cancelled successfully: runID=%s", featureItem.FlowRunID)
	}

	// Clear the run ID
	if err := s.ClearFlowRunID(ctx, mainItem.ID); err != nil {
		log.Warnfc(ctx, "failed to clear flow run id: %v", err)
	}

	// Add a comment to notify about the cancellation
	_ = s.CMS.CommentToItem(ctx, mainItem.ID, "Flowジョブをキャンセルしました。")

	return nil
}
