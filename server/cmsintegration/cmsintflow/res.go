package cmsintflow

import (
	"context"
	"encoding/json"
	"fmt"
	"sort"
	"strings"

	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
	"github.com/k0kubun/pp/v3"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/log"
	"github.com/samber/lo"
	"golang.org/x/exp/maps"
)

func receiveResultFromFlow(ctx context.Context, s *Services, conf *Config, res FlowResult) error {
	id, err := parseID(res.ID, conf.Secret)
	if err != nil {
		log.Debugfc(ctx, "failed to parse id: %s", res.ID)
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
		log.Debugfc(ctx, "failed to convert: logs=%v", res.Logs)
		_ = s.Fail(ctx, id.ItemID, cmsintegrationcommon.ReqType(id.Type), "%sに失敗しました。%s", cmsintegrationcommon.ReqType(id.Type).Title(), logurls)
		return nil
	}

	// feature types
	featureTypes, err := s.PCMS.PlateauFeatureTypes(ctx)
	if err != nil {
		log.Errorfc(ctx, "failed to get feature types: %v", err)
		return nil
	}

	featureType, ok := lo.Find(featureTypes, func(ft plateaucms.PlateauFeatureType) bool {
		return ft.Code == id.FeatureType
	})
	if !ok {
		log.Debugfc(ctx, "invalid feature type: %s", id.FeatureType)
		return nil
	}

	// get mainItem
	mainItem, err := s.CMS.GetItem(ctx, id.ItemID, false)
	if err != nil {
		log.Debugfc(ctx, "failed to get item: %v", err)
		return fmt.Errorf("failed to get item: %w", err)
	}

	baseFeatureItem := cmsintegrationcommon.FeatureItemFrom(mainItem)

	// outputs
	internal := res.Internal()

	// upload assets
	log.Infofc(ctx, "upload assets")
	var dataAssets []string
	dataAssetMap := map[string][]string{}
	dataAssets = make([]string, 0, len(res.Outputs))

	for key, urls := range internal.Conv {
		for _, u := range urls {
			aid, err := s.UploadAsset(ctx, id.ProjectID, u)
			if err != nil {
				log.Errorfc(ctx, "failed to upload asset (%s): %v", u, err)
				return nil
			}
			dataAssets = append(dataAssets, aid)
			dataAssetMap[key] = append(dataAssetMap[key], aid)
		}
	}

	// read dic
	var dic string
	if internal.Dic != "" {
		var err error
		log.Debugfc(ctx, "read dic: %s", internal.Dic)
		dic, err = readDic(ctx, internal.Dic)
		if err != nil {
			log.Errorfc(ctx, "failed to read dic: %v", err)
			return nil
		}
	}

	// upload maxlod
	var maxlodAssetID string
	if internal.MaxLOD != "" {
		log.Debugfc(ctx, "upload maxlod: %s", internal.MaxLOD)
		var err error
		maxlodAssetID, err = s.UploadAsset(ctx, id.ProjectID, internal.MaxLOD)
		if err != nil {
			return fmt.Errorf("failed to upload maxlod: %w", err)
		}
	}

	// upload qc result
	var qcResult string
	if internal.QCResult != "" {
		log.Debugfc(ctx, "upload qc result: %s", internal.QCResult)
		var err error
		qcResult, err = s.UploadAsset(ctx, id.ProjectID, internal.QCResult)
		if err != nil {
			return fmt.Errorf("failed to upload qc result: %w", err)
		}
	}

	// update item
	qcStatus, convStatus := id.Type.CMSStatus(cmsintegrationcommon.ConvertionStatusSuccess)

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
		MaxLOD:           maxlodAssetID,
		QCResult:         qcResult,
		ConvertionStatus: cmsintegrationcommon.TagFrom(convStatus),
		QCStatus:         cmsintegrationcommon.TagFrom(qcStatus),
	}).CMSItem()

	log.Debugfc(ctx, "update item: %s", pp.Sprint(newitem))

	_, err = s.CMS.UpdateItem(ctx, id.ItemID, newitem.Fields, newitem.MetadataFields)
	if err != nil {
		j1, _ := json.Marshal(newitem.Fields)
		j2, _ := json.Marshal(newitem.MetadataFields)
		log.Debugfc(ctx, "item update for %s: %s, %s", id.ItemID, j1, j2)
		log.Errorfc(ctx, "failed to update item: %v", err)
		return fmt.Errorf("failed to update item: %w", err)
	}

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

	// if the qc is success, trigger the conversion
	if id.Type == cmsintegrationcommon.ReqTypeQC && qcStatus == cmsintegrationcommon.ConvertionStatusSuccess {
		log.Infofc(ctx, "trigger conv")
		rewriteQCStatus(mainItem, cmsintegrationcommon.ConvertionStatusSuccess)
		if err := sendRequestToFlow(ctx, s, conf, id.ProjectID, featureType.Code, mainItem, featureTypes, cmsintegrationcommon.ReqTypeConv); err != nil {
			log.Errorfc(ctx, "failed to trigger conv: %v", err)
			return fmt.Errorf("failed to send request to flow: %w", err)
		}
	}

	return nil
}

func getFeatureItemData(assets map[string][]string, items []cmsintegrationcommon.FeatureItemDatum) (res []cmsintegrationcommon.FeatureItemDatum) {
	keys := maps.Keys(assets)
	sort.Strings(keys)

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
