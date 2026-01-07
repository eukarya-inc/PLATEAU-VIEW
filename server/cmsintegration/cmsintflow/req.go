package cmsintflow

import (
	"context"
	"fmt"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/k0kubun/pp/v3"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/log"
	"github.com/samber/lo"
)

func sendRequestToFlow(
	ctx context.Context,
	s *Services,
	conf *Config,
	projectID string,
	modelName string,
	mainItem *cms.Item,
	featureTypes plateaucms.PlateauFeatureTypeList,
	overrideReqType cmsintegrationcommon.ReqType,
) error {
	ctx = log.WithPrefixMessage(ctx, "flow: ")

	item := cmsintegrationcommon.FeatureItemFrom(mainItem)
	log.Debugfc(ctx, "item: %s", pp.Sprint(item))

	if item == nil {
		log.Infofc(ctx, "skip: item is nil (mainItem.ID=%s)", mainItem.ID)
		return nil
	}

	if item.CityGML == "" || item.City == "" {
		log.Infofc(ctx, "skip: no city or no citygml: city=%s, citygml=%s", item.City, item.CityGML)
		return nil
	}

	// feature type
	featureTypeCodes := featureTypes.Codes()
	featureTypeCode := item.FeatureTypeCode()
	featureType, ok := lo.Find(featureTypes, func(ft plateaucms.PlateauFeatureType) bool {
		return ft.Code == modelName || ft.Code == featureTypeCode
	})
	if !ok {
		log.Infofc(ctx, "skip: invalid feature type or model name: modelName=%s, featureTypeCode=%s, availableCodes=%v", modelName, featureTypeCode, featureTypeCodes)
		return nil
	}

	log.Debugfc(ctx, "feature type: %s", pp.Sprint(featureType))
	if !featureType.Conv && !featureType.QC {
		log.Infofc(ctx, "skip: feature type does not support qc or convert: code=%s, conv=%v, qc=%v", featureType.Code, featureType.Conv, featureType.QC)
		return nil
	}

	// type
	fty := cmsintegrationcommon.ReqTypeFrom(!featureType.QC, !featureType.Conv)
	ity := item.ReqType().Override(overrideReqType)
	ty := fty.Intersection(ity).Normalize()
	if ty == "" || ty == cmsintegrationcommon.ReqTypeQCConv {
		log.Infofc(ctx, "skip: request type is empty or qc_conv: fty=%s, ity=%s, ty=%s", fty, ity, ty)
		return nil
	}

	log.Infofc(ctx, "processing: item=%s, featureType=%s, reqType=%s", mainItem.ID, featureType.Code, ty)

	// update convertion status
	if err := s.UpdateFeatureItemStatus(ctx, mainItem.ID, ty, cmsintegrationcommon.ConvertionStatusRunning); err != nil {
		log.Errorfc(ctx, "failed to update item status: %v", err)
		return fmt.Errorf("failed to update item: %w", err)
	}

	// get CityGML asset
	log.Debugfc(ctx, "getting citygml asset: %s", item.CityGML)
	cityGMLAsset, err := s.CMS.Asset(ctx, item.CityGML)
	if err != nil {
		log.Errorfc(ctx, "failed to get citygml asset: id=%s, err=%v", item.CityGML, err)
		_ = s.Fail(ctx, mainItem.ID, ty, "CityGMLが見つかりません。")
		return fmt.Errorf("failed to get citygml asset: %w", err)
	}
	log.Debugfc(ctx, "citygml asset url: %s", cityGMLAsset.URL)

	// get city item
	log.Debugfc(ctx, "getting city item: %s", item.City)
	cityItemRaw, err := s.CMS.GetItem(ctx, item.City, true)
	if err != nil {
		log.Errorfc(ctx, "failed to get city item: id=%s, err=%v", item.City, err)
		_ = s.Fail(ctx, mainItem.ID, ty, "都市アイテムが正常に紐づけられていません。")
		return fmt.Errorf("failed to get city item: %w", err)
	}

	cityItem := cmsintegrationcommon.CityItemFrom(cityItemRaw, featureTypeCodes)
	log.Debugfc(ctx, "city item: %s", pp.Sprint(cityItem))

	// specv - prioritize item spec over city spec
	specv := item.SpecMajorVersionInt()
	if specv == 0 {
		specv = cityItem.SpecMajorVersionInt()
	}
	log.Debugfc(ctx, "specv: item=%d, city=%d, final=%d", item.SpecMajorVersionInt(), cityItem.SpecMajorVersionInt(), specv)
	if specv == 0 {
		log.Errorfc(ctx, "specv is 0: item specv=%d, city specv=%d", item.SpecMajorVersionInt(), cityItem.SpecMajorVersionInt())
		_ = s.Fail(ctx, mainItem.ID, ty, "仕様書バージョンを指定してください。")
		return fmt.Errorf("failed to get specv: specv=%d", specv)
	}

	// trigger id
	var qc bool
	log.Debugfc(ctx, "status: ty=%s, overrideReqType=%s, specv=%d", ty, overrideReqType, specv)
	var triggerID string
	switch ty {
	case cmsintegrationcommon.ReqTypeQC:
		triggerID = featureType.FlowQCTriggerID(specv)
		qc = true
	case cmsintegrationcommon.ReqTypeConv:
		triggerID = featureType.FlowConvTriggerID(specv)
	}
	log.Debugfc(ctx, "trigger id: %s (ty=%s, specv=%d, qc=%v)", triggerID, ty, specv, qc)
	if triggerID == "" {
		log.Errorfc(ctx, "trigger id is empty: ty=%s, specv=%d, featureType=%s", ty, specv, featureType.Code)
		_ = s.Fail(ctx, mainItem.ID, ty, "Flowの%s（v%d）用トリガーIDが設定されていません。", ty.Title(), specv)
		return fmt.Errorf("failed to get trigger id: ty=%s, v=%d", ty, specv)
	}

	// conv settings
	convSettings := cityItem.ConvSettings().Merge(item.ConvSettings())
	if convSettings != nil && convSettings.FeatureType == "" {
		convSettings.FeatureType = featureType.Code
	}
	log.Debugfc(ctx, "conv settings: %s", pp.Sprint(convSettings))
	if err := convSettings.Validate(qc); err != nil {
		log.Errorfc(ctx, "invalid conv settings: %v", err)
		_ = s.Fail(ctx, mainItem.ID, ty, "%v", err)
		return fmt.Errorf("invalid conv settings: %w", err)
	}

	// sign id
	sig := ID{
		ItemID:      mainItem.ID,
		ProjectID:   projectID,
		FeatureType: featureType.Code,
		Type:        ty,
	}.Sign(conf.Secret)

	// request to flow
	notificationURL := resultURL(conf, sig)
	log.Infofc(ctx, "requesting to flow: triggerID=%s, notificationURL=%s, cityGMLURL=%s", triggerID, notificationURL, cityGMLAsset.URL)
	res, err := s.Flow.Request(ctx, FlowRequest{
		TriggerID:       triggerID,
		NotificationURL: notificationURL,
		AuthToken:       conf.FlowToken,
		CityGMLURL:      cityGMLAsset.URL,
		ConvSettings:    convSettings,
	})
	if err != nil {
		log.Errorfc(ctx, "failed to request to flow: triggerID=%s, err=%v", triggerID, err)
		_ = s.Fail(ctx, mainItem.ID, ty, "Flowへのリクエストに失敗しました。%v", err)
		return fmt.Errorf("failed to request to flow: %w", err)
	}

	log.Infofc(ctx, "success to trigger flow: item=%s, triggerID=%s, res=%#v", mainItem.ID, triggerID, res)

	// post a comment to the item
	if err = s.CMS.CommentToItem(ctx, mainItem.ID, fmt.Sprintf("Flowでの%s（v%d）を開始しました。", ty.Title(), specv)); err != nil {
		return fmt.Errorf("failed to add comment: %w", err)
	}

	return nil
}
