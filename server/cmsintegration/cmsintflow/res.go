package cmsintflow

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"maps"
	"slices"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/k0kubun/pp/v3"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/log"
	"github.com/samber/lo"
)

func receiveResultFromFlow(ctx context.Context, s *Services, conf *Config, res FlowResult) error {
	log.Infofc(ctx, "receiveResultFromFlow start: res=%s", pp.Sprint(res))

	id, err := parseID(res.ID, conf.Secret)
	if err != nil {
		log.Infofc(ctx, "early return: failed to parse id: %s, err=%v", res.ID, err)
		return nil
	}

	log.Infofc(ctx, "id: %#v", id)

	// log urls
	logurls := strings.Join(res.Logs, "\n")
	if logurls != "" {
		logurls = "ログ: " + logurls
	}

	// handle error
	if res.IsFailed() {
		log.Infofc(ctx, "early return: flow result is failed: status=%s, logs=%v", res.Status, res.Logs)
		failErr := s.Fail(ctx, id.ItemID, cmsintegrationcommon.ReqType(id.Type), "%sに失敗しました。%s%s", cmsintegrationcommon.ReqType(id.Type).Title(), res.IDsMessage(), logurls)
		// Clear runId on failure
		if err := errors.Join(failErr, s.ClearFlowRunID(ctx, id.ItemID)); err != nil {
			return fmt.Errorf("failed to record flow failure: %w", err)
		}
		return nil
	}

	// feature types
	featureTypes, err := s.PCMS.PlateauFeatureTypes(ctx)
	if err != nil {
		log.Errorfc(ctx, "failed to get feature types: %v", err)
		return s.abortResult(ctx, id, res, fmt.Errorf("failed to get feature types: %w", err))
	}

	// For derived feature types (e.g., bldg2), use the base feature type (bldg) from plateau-features
	baseFeatureType := cmsintegrationcommon.ExtractBaseFeatureType(id.FeatureType)
	featureType, ok := lo.Find(featureTypes, func(ft plateaucms.PlateauFeatureType) bool {
		return ft.Code == id.FeatureType || ft.Code == baseFeatureType
	})
	if !ok {
		log.Infofc(ctx, "early return: invalid feature type: %s (base=%s), available=%v", id.FeatureType, baseFeatureType, lo.Map(featureTypes, func(ft plateaucms.PlateauFeatureType, _ int) string { return ft.Code }))
		return nil
	}

	// plateau specs
	specs, err := s.PCMS.PlateauSpecs(ctx)
	if err != nil {
		log.Errorfc(ctx, "failed to get plateau specs: %v", err)
		return s.abortResult(ctx, id, res, fmt.Errorf("failed to get plateau specs: %w", err))
	}

	// get mainItem
	mainItem, err := s.CMS.GetItem(ctx, id.ItemID, false)
	if err != nil {
		log.Infofc(ctx, "failed to get item: itemID=%s, err=%v", id.ItemID, err)
		return fmt.Errorf("failed to get item: %w", err)
	}
	log.Infofc(ctx, "mainItem: %s", pp.Sprint(mainItem))

	baseFeatureItem := cmsintegrationcommon.FeatureItemFrom(mainItem)
	log.Infofc(ctx, "baseFeatureItem: %s", pp.Sprint(baseFeatureItem))

	// outputs
	internal := res.Internal()
	log.Infofc(ctx, "internal: %s", pp.Sprint(internal))

	// upload assets
	log.Infofc(ctx, "upload assets")
	var dataAssets []string
	dataAssetMap := map[string][]string{}
	dataAssetURLs := map[string]string{}
	dataAssets = make([]string, 0, len(res.Outputs))

	for key, urls := range internal.Conv {
		for _, u := range urls {
			aid, err := s.UploadAsset(ctx, id.ProjectID, u)
			if err != nil {
				log.Errorfc(ctx, "failed to upload asset: key=%s, url=%s, err=%v", key, u, err)
				return s.abortResult(ctx, id, res, fmt.Errorf("failed to upload asset (%s): %w", u, err))
			}
			log.Infofc(ctx, "uploaded asset: key=%s, url=%s, assetID=%s", key, u, aid)
			dataAssets = append(dataAssets, aid)
			dataAssetMap[key] = append(dataAssetMap[key], aid)
			dataAssetURLs[aid] = u
		}
	}

	// check if conversion has no assets
	if id.Type == cmsintegrationcommon.ReqTypeConv && len(dataAssets) == 0 {
		log.Infofc(ctx, "no assets returned from flow for conversion")
		_ = s.CMS.CommentToItem(ctx, id.ItemID, fmt.Sprintf("Flowから変換結果のアセットが返されませんでした。%sFlowのログを確認してください。", res.IDsMessage()))
		return nil
	}

	// read dic
	var dic string
	if internal.Dic != "" {
		var err error
		log.Infofc(ctx, "read dic: %s", internal.Dic)
		dic, err = readDic(ctx, internal.Dic)
		if err != nil {
			log.Errorfc(ctx, "failed to read dic: url=%s, err=%v", internal.Dic, err)
			return s.abortResult(ctx, id, res, fmt.Errorf("failed to read dic: %w", err))
		}
		log.Infofc(ctx, "dic read success: len=%d", len(dic))
	}

	// upload qc result
	var qcResult string
	if internal.QCResult != "" {
		log.Infofc(ctx, "upload qc result: %s", internal.QCResult)
		var err error
		qcResult, err = s.UploadAsset(ctx, id.ProjectID, internal.QCResult)
		if err != nil {
			log.Infofc(ctx, "failed to upload qc result: url=%s, err=%v", internal.QCResult, err)
			return fmt.Errorf("failed to upload qc result: %w", err)
		}
		log.Infofc(ctx, "qc result uploaded: assetID=%s", qcResult)
	}

	// Determine status updates and next actions via workflow state machine
	wm := NewWorkflowMachine(baseFeatureItem, featureType.QC, featureType.Conv)
	var wmEvent Event
	switch id.Type {
	case cmsintegrationcommon.ReqTypeQC:
		wmEvent = Event{Kind: EventQCCompleted, QCOK: internal.QCOK}
	case cmsintegrationcommon.ReqTypeConv:
		wmEvent = Event{Kind: EventConvCompleted}
	}
	wmActions, _ := wm.Transition(wmEvent)
	log.Debugfc(ctx, "workflow: event=%+v, actions=%+v, newState=%+v", wmEvent, wmActions, wm.State)

	// Extract status values from workflow actions for the CMS item update
	var qcStatus, convStatus cmsintegrationcommon.ConvertionStatus
	var shouldStartConv bool
	for _, a := range wmActions {
		switch a.Kind {
		case ActionSetStatus:
			if a.QCStatus != "" {
				qcStatus = a.QCStatus
			}
			if a.ConvStatus != "" {
				convStatus = a.ConvStatus
			}
		case ActionStartConv:
			shouldStartConv = true
		}
	}

	// items
	// The converted assets replace only the existing assets of the same CRS, so that the
	// JGD2011 and JGD2024 versions can be kept side by side.
	var data []string
	var items []cmsintegrationcommon.FeatureItemDatum
	if len(dataAssets) > 0 {
		names, err := convertedAssetNames(ctx, s.CMS, baseFeatureItem, dataAssetURLs)
		if err != nil {
			log.Infofc(ctx, "failed to get asset names: itemID=%s, err=%v", id.ItemID, err)
			return fmt.Errorf("failed to get asset names: %w", err)
		}

		if featureType.UseGroups {
			keys := slices.Sorted(maps.Keys(dataAssetMap))
			items, err = cmsintegrationcommon.MergeConvertedItems(baseFeatureItem.Items, dataAssetMap, keys, names, cmsintegrationcommon.GenerateCMSID)
		} else {
			data, err = cmsintegrationcommon.MergeConvertedAssets(baseFeatureItem.Data, dataAssets, names)
		}
		if err != nil {
			log.Infofc(ctx, "early return: failed to merge converted assets: %v", err)
			_ = s.Fail(ctx, id.ItemID, cmsintegrationcommon.ReqType(id.Type), "変換結果をアイテムに反映できませんでした。%v%s", err, res.IDsMessage())
			_ = s.ClearFlowRunID(ctx, id.ItemID)
			return nil
		}
	}

	newitem := (&cmsintegrationcommon.FeatureItem{
		Data:             data,
		Items:            items,
		Dic:              dic,
		QCResult:         qcResult,
		ConvertionStatus: cmsintegrationcommon.TagFrom(convStatus),
		QCStatus:         cmsintegrationcommon.TagFrom(qcStatus),
	}).CMSItem()

	// Remove empty data/items fields to preserve existing data
	newitem.Fields = lo.Filter(newitem.Fields, func(f *cms.Field, _ int) bool {
		if f.Key == "data" {
			if v, ok := f.Value.([]string); ok && len(v) == 0 {
				return false
			}
		}
		if f.Key == "items" {
			// items field uses group type, check if items slice is empty
			if len(items) == 0 {
				return false
			}
		}
		return true
	})

	log.Infofc(ctx, "update item: itemID=%s, newitem=%s", id.ItemID, pp.Sprint(newitem))
	j1, _ := json.Marshal(newitem.Fields)
	j2, _ := json.Marshal(newitem.MetadataFields)
	log.Infofc(ctx, "update item JSON: fields=%s, metadataFields=%s", j1, j2)

	updatedItem, err := s.CMS.UpdateItem(ctx, id.ItemID, newitem.Fields, newitem.MetadataFields)
	if err != nil {
		log.Infofc(ctx, "failed to update item: itemID=%s, err=%v", id.ItemID, err)
		return fmt.Errorf("failed to update item: %w", err)
	}
	log.Infofc(ctx, "update item success: response=%s", pp.Sprint(updatedItem))

	// comment to the item
	qcmsg := ""
	if id.Type == cmsintegrationcommon.ReqTypeQC {
		if internal.QCOK {
			qcmsg = ""
		} else {
			qcmsg = "品質検査でエラーが検出されました。"
		}
	}
	if err := s.CMS.CommentToItem(ctx, id.ItemID, fmt.Sprintf("Flowの%sが完了しました。%s%s", id.Type.Title(), qcmsg, logurls)); err != nil {
		return fmt.Errorf("failed to add comment: %w", err)
	}

	log.Infofc(ctx, "success to receive result from flow: %s", id.Type)

	// Clear runId on completion (will be set again if conv is triggered after QC)
	_ = s.ClearFlowRunID(ctx, id.ItemID)

	// If the workflow state machine decided to start conv (after QC success), trigger it
	if shouldStartConv {
		log.Infofc(ctx, "trigger conv after QC success")
		rewriteQCStatus(mainItem, cmsintegrationcommon.ConvertionStatusSuccess)
		if err := sendRequestToFlow(ctx, s, conf, id.ProjectID, featureType.Code, mainItem, featureTypes, plateaucms.PlateauSpecList(specs), cmsintegrationcommon.ReqTypeConv); err != nil {
			log.Errorfc(ctx, "failed to trigger conv: %v", err)
			return fmt.Errorf("failed to send request to flow: %w", err)
		}
	}

	return nil
}

func rewriteQCStatus(item *cms.Item, status cmsintegrationcommon.ConvertionStatus) {
	if item == nil {
		return
	}
	tag := cmsintegrationcommon.TagFrom(status)
	// qc_status is a metadata field, so update MetadataFields
	for i, f := range item.MetadataFields {
		if f.Key == "qc_status" {
			item.MetadataFields[i].Value = tag
			return
		}
	}
	// fallback: also check Fields for backward compatibility
	for i, f := range item.Fields {
		if f.Key == "qc_status" {
			item.Fields[i].Value = tag
			return
		}
	}
}

// convertedAssetNames returns the names of the existing data assets of the item and the
// URLs of the converted assets, keyed by asset ID.
func convertedAssetNames(ctx context.Context, c cmsintegrationcommon.AssetGetter, item *cmsintegrationcommon.FeatureItem, converted map[string]string) (map[string]string, error) {
	ids := slices.Clone(item.Data)
	for _, d := range item.Items {
		ids = append(ids, d.Data...)
	}
	names, err := cmsintegrationcommon.AssetNames(ctx, c, ids)
	if err != nil {
		return nil, err
	}
	maps.Copy(names, converted)
	return names, nil
}

// abortResult marks the item as failed and clears its run ID when a result
// can't be ingested, and returns cause so the callback answers 500. Returning
// nil here used to ack the result while the item sat at "running" forever and,
// for QC, the follow-up conversion never started.
func (s *Services) abortResult(ctx context.Context, id ID, res FlowResult, cause error) error {
	ty := cmsintegrationcommon.ReqType(id.Type)
	failErr := s.Fail(ctx, id.ItemID, ty, "%sの結果の取り込みに失敗しました。再実行してください。%s\n%v", ty.Title(), res.IDsMessage(), cause)
	if err := errors.Join(failErr, s.ClearFlowRunID(ctx, id.ItemID)); err != nil {
		return errors.Join(cause, fmt.Errorf("failed to record failure: %w", err))
	}
	return cause
}
