package cmsintflow

import (
	"context"
	"encoding/json"
	"fmt"
	"maps"
	"slices"
	"sort"
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
		_ = s.Fail(ctx, id.ItemID, cmsintegrationcommon.ReqType(id.Type), "%sに失敗しました。%s%s", cmsintegrationcommon.ReqType(id.Type).Title(), res.IDsMessage(), logurls)
		return nil
	}

	// feature types
	featureTypes, err := s.PCMS.PlateauFeatureTypes(ctx)
	if err != nil {
		log.Infofc(ctx, "early return: failed to get feature types: %v", err)
		return nil
	}

	featureType, ok := lo.Find(featureTypes, func(ft plateaucms.PlateauFeatureType) bool {
		return ft.Code == id.FeatureType
	})
	if !ok {
		log.Infofc(ctx, "early return: invalid feature type: %s, available=%v", id.FeatureType, lo.Map(featureTypes, func(ft plateaucms.PlateauFeatureType, _ int) string { return ft.Code }))
		return nil
	}

	// plateau specs
	specs, err := s.PCMS.PlateauSpecs(ctx)
	if err != nil {
		log.Infofc(ctx, "early return: failed to get plateau specs: %v", err)
		return nil
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
	internal := res.InternalWithFeatureType(featureType.Code, featureType.UseGroups)
	log.Infofc(ctx, "internal: %s", pp.Sprint(internal))

	// upload assets
	log.Infofc(ctx, "upload assets")
	var dataAssets []string
	dataAssetMap := map[string][]string{}
	dataAssets = make([]string, 0, len(res.Outputs))

	for key, urls := range internal.Conv {
		for _, u := range urls {
			aid, err := s.UploadAsset(ctx, id.ProjectID, u)
			if err != nil {
				log.Infofc(ctx, "early return: failed to upload asset: key=%s, url=%s, err=%v", key, u, err)
				return nil
			}
			log.Infofc(ctx, "uploaded asset: key=%s, url=%s, assetID=%s", key, u, aid)
			dataAssets = append(dataAssets, aid)
			dataAssetMap[key] = append(dataAssetMap[key], aid)
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
			log.Infofc(ctx, "early return: failed to read dic: url=%s, err=%v", internal.Dic, err)
			return nil
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

	// update item
	qcStatus, convStatus := id.Type.CMSStatus(cmsintegrationcommon.ConvertionStatusSuccess)

	// if QC detected errors, set qcStatus to error
	if id.Type == cmsintegrationcommon.ReqTypeQC && !internal.QCOK {
		qcStatus = cmsintegrationcommon.ConvertionStatusError
		log.Infofc(ctx, "QC detected errors, setting qcStatus to error")
	}

	// items
	var data []string
	var items []cmsintegrationcommon.FeatureItemDatum
	if featureType.UseGroups {
		items = getFeatureItemData(dataAssetMap, baseFeatureItem.Items)
	} else {
		sort.Strings(dataAssets)
		data = dataAssets
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

	// if the qc is success and QCOK, trigger the conversion (unless conversion is skipped)
	if id.Type == cmsintegrationcommon.ReqTypeQC && qcStatus == cmsintegrationcommon.ConvertionStatusSuccess {
		if !internal.QCOK {
			log.Infofc(ctx, "skip conv after qc because QC detected errors (QCOK=false)")
		} else {
			// Check if conversion should be skipped
			_, skipConv := baseFeatureItem.IsQCAndConvSkipped()
			if skipConv || !featureType.Conv {
				log.Infofc(ctx, "skip conv after qc success because conversion is marked as skip or feature type doesn't support conversion")
			} else {
				log.Infofc(ctx, "trigger conv")
				rewriteQCStatus(mainItem, cmsintegrationcommon.ConvertionStatusSuccess)
				if err := sendRequestToFlow(ctx, s, conf, id.ProjectID, featureType.Code, mainItem, featureTypes, plateaucms.PlateauSpecList(specs), cmsintegrationcommon.ReqTypeConv); err != nil {
					log.Errorfc(ctx, "failed to trigger conv: %v", err)
					return fmt.Errorf("failed to send request to flow: %w", err)
				}
			}
		}
	}

	return nil
}

func getFeatureItemData(assets map[string][]string, items []cmsintegrationcommon.FeatureItemDatum) (res []cmsintegrationcommon.FeatureItemDatum) {
	keys := slices.Sorted(maps.Keys(assets))

	for _, k := range keys {
		assets := assets[k]
		i, ok := lo.Find(items, func(i cmsintegrationcommon.FeatureItemDatum) bool {
			return i.Key == k
		})

		var id string
		if ok {
			id = i.ID
		} else {
			id = cmsintegrationcommon.GenerateCMSID()
		}

		res = append(res, cmsintegrationcommon.FeatureItemDatum{
			ID:   id,
			Data: assets,
			Key:  k,
		})
	}

	return
}

func rewriteQCStatus(item *cms.Item, status cmsintegrationcommon.ConvertionStatus) {
	if item == nil {
		return
	}
	for i, f := range item.Fields {
		if f.Key == "qc_status" {
			item.Fields[i].Value = cmsintegrationcommon.TagFrom(status)
			return
		}
	}
}
