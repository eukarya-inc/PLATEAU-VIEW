package cmsintegrationv3

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"sort"
	"strings"

	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
	"github.com/k0kubun/pp/v3"
	"github.com/oklog/ulid/v2"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/log"
	"github.com/samber/lo"
	"github.com/spkg/bom"
)

const plateauSpecMinMajorVersionObjectListsNeeded = 4
const flowTestModel = "flow"

var generateID = func() string {
	return strings.ToLower(ulid.Make().String())
}

func sendRequestToFME(ctx context.Context, s *Services, conf *Config, w *cmswebhook.Payload) error {
	ctx = log.WithPrefixMessage(ctx, "fme: ")

	// if event type is "item.create" and payload is metadata, skip it
	if w.Type == cmswebhook.EventItemCreate && (w.ItemData.Item.OriginalItemID != nil || w.ItemData.Item.IsMetadata) {
		return nil
	}

	mainItem, err := s.GetMainItemWithMetadata(ctx, w.ItemData.Item)
	if err != nil {
		return err
	}

	// metadata
	md, _, err := s.PCMS.Metadata(ctx, w.ProjectID(), false, false)
	if err != nil {
		log.Errorfc(ctx, "failed to get metadata: %v", err)
		return nil
	}
	if !md.IsFMEEnabled() {
		log.Debugfc(ctx, "fme is disabled")
		return nil
	}

	// feature item
	item := cmsintegrationcommon.FeatureItemFrom(mainItem)

	featureTypeCode := strings.TrimPrefix(w.ItemData.Model.Key, cmsintegrationcommon.ModelPrefix)
	if featureTypeCode == flowTestModel {
		log.Debugfc(ctx, "skip model: %s", featureTypeCode)
		return nil
	}
	if ft := item.FeatureTypeCode(); ft != "" {
		featureTypeCode = ft
		log.Debugfc(ctx, "feature type is overridden: %s", ft)
	}

	// feature types
	featureTypes, err := s.PCMS.PlateauFeatureTypes(ctx)
	if err != nil {
		log.Errorfc(ctx, "failed to get feature types: %v", err)
		return nil
	}

	featureTypeCodes := featureTypes.Codes()
	featureType, ok := lo.Find(featureTypes, func(ft plateaucms.PlateauFeatureType) bool {
		return ft.Code == featureTypeCode
	})
	if !ok {
		log.Debugfc(ctx, "invalid feature item: %s", featureTypeCode)
		return nil
	}

	log.Debugfc(ctx, "featureType: %s", pp.Sprint(featureType))
	if !featureType.QC && !featureType.Conv {
		log.Debugfc(ctx, "skip qc and convert by feature type")
		return nil
	}

	skipQC, skipConv := item.IsQCAndConvSkipped()
	if skipQC && skipConv {
		log.Debugfc(ctx, "skip qc and convert by item")
		return nil
	}

	if item.CityGML == "" || item.City == "" {
		log.Debugfc(ctx, "no city or no citygml")
		return nil
	}

	ty := fmeTypeQcConv
	if !featureType.QC || skipQC {
		ty = fmeTypeConv
	} else if !featureType.Conv || skipConv {
		ty = fmeTypeQC
	}

	log.Debugfc(ctx, "itemID=%s featureType=%s", mainItem.ID, featureTypeCode)
	log.Debugfc(ctx, "raw item: %s", pp.Sprint(mainItem))
	log.Debugfc(ctx, "item: %s", pp.Sprint(item))

	// update convertion status
	err = s.UpdateFeatureItemStatus(ctx, mainItem.ID, ty, cmsintegrationcommon.ConvertionStatusRunning)
	if err != nil {
		return fmt.Errorf("failed to update item: %w", err)
	}

	// get CityGML asset
	cityGMLAsset, err := s.CMS.Asset(ctx, item.CityGML)
	if err != nil {
		_ = failToConvert(ctx, s, mainItem.ID, ty, "CityGMLが見つかりません。")
		return fmt.Errorf("failed to get citygml asset: %w", err)
	}

	// get city item
	cityItemRaw, err := s.CMS.GetItem(ctx, item.City, true)
	if err != nil {
		_ = failToConvert(ctx, s, mainItem.ID, ty, "都市アイテムが見つかりません。")
		return fmt.Errorf("failed to get city item: %w", err)
	}

	cityItem := cmsintegrationcommon.CityItemFrom(cityItemRaw, featureTypeCodes)

	// validate city item
	if cityItem.CodeLists == nil {
		_ = failToConvert(ctx, s, mainItem.ID, ty, "コードリストが都市アイテムに登録されていないため品質検査・変換を開始できません。")
		return fmt.Errorf("city item has no codelist")
	}

	if !skipQC && cityItem.ObjectLists == nil && cityItem.SpecMajorVersionInt() >= plateauSpecMinMajorVersionObjectListsNeeded {
		_ = failToConvert(ctx, s, mainItem.ID, ty, "オブジェクトリストが都市アイテムに登録されていないため品質検査を開始できません。")
		return fmt.Errorf("city item has no objectlists")
	}

	// get objectLists asset
	var objectListsAssetURL string
	if cityItem.ObjectLists != nil {
		objectListsAssetURL = cityItem.ObjectLists.URL
	}

	// get FME URL
	fme := s.GetFME(s.GetFMEURL(ctx, cityItem.SpecMajorVersionInt()))
	if fme == nil {
		_ = failToConvert(ctx, s, mainItem.ID, ty, "FMEのURLが設定されていません。")
		return fmt.Errorf("fme url is not set")
	}

	// request to fme
	err = fme.Request(ctx, fmeRequest{
		ID: fmeID{
			ItemID:      mainItem.ID,
			ProjectID:   w.ProjectID(),
			FeatureType: featureTypeCode,
			Type:        string(ty),
		}.String(conf.Secret),
		Target:      cityGMLAsset.URL,
		PRCS:        cityItem.PRCS.EPSGCode(),
		Codelists:   cityItem.CodeLists.URL,
		ObjectLists: objectListsAssetURL,
		ResultURL:   resultURL(conf),
		Type:        ty,
	})
	if err != nil {
		_ = failToConvert(ctx, s, mainItem.ID, ty, "FMEへのリクエストに失敗しました。%v", err)
		return fmt.Errorf("failed to request to fme: %w", err)
	}

	// post a comment to the item
	err = s.CMS.CommentToItem(ctx, mainItem.ID, fmt.Sprintf("%sを開始しました。", ty.Title()))
	if err != nil {
		return fmt.Errorf("failed to add comment: %w", err)
	}

	log.Infofc(ctx, "sendRequestToFME: success")
	return nil
}

func receiveResultFromFME(ctx context.Context, s *Services, conf *Config, f fmeResult) error {
	ctx = log.WithPrefixMessage(ctx, "receiveResultFromFME: ")
	id := f.ParseID(conf.Secret)
	if id.ItemID == "" {
		return fmt.Errorf("invalid id: %s", f.ID)
	}

	log.Infofc(ctx, "itemID=%s featureType=%s type=%s", id.ItemID, id.FeatureType, id.Type)

	logmsg := f.Message
	if f.LogURL != "" {
		if logmsg != "" {
			logmsg += " "
		}
		logmsg += "ログ： " + f.LogURL
	}

	// notify
	if f.Type == "notify" {
		log.Debugfc(ctx, "notify: %s", logmsg)

		if err := s.CMS.CommentToItem(ctx, id.ItemID, logmsg); err != nil {
			return fmt.Errorf("failed to comment: %w", err)
		}

		// upload qc result
		if f.Results != nil {
			if qcResult, ok := f.Results["_qc_result"]; ok {
				if qcResultStr, ok := qcResult.(string); ok {
					log.Debugfc(ctx, "upload qc result: %s", qcResultStr)
					var err error
					qcResultAsset, err := s.UploadAsset(ctx, id.ProjectID, qcResultStr)
					if err != nil {
						return fmt.Errorf("failed to upload maxlod: %w", err)
					}

					item := (&cmsintegrationcommon.FeatureItem{
						QCStatus: cmsintegrationcommon.TagFrom(cmsintegrationcommon.ConvertionStatusSuccess),
						QCResult: qcResultAsset,
					}).CMSItem()

					_, err = s.CMS.UpdateItem(ctx, id.ItemID, item.Fields, item.MetadataFields)
					if err != nil {
						j1, _ := json.Marshal(item.Fields)
						j2, _ := json.Marshal(item.MetadataFields)
						log.Debugfc(ctx, "item update for %s: %s, %s", id.ItemID, j1, j2)
						log.Errorfc(ctx, "failed to update item: %v", err)
						return fmt.Errorf("failed to update item: %w", err)
					}
				}
			}
		}

		return nil
	}

	// handle error
	if f.Status == "error" {
		log.Warnfc(ctx, "failed to convert: %v", f.LogURL)
		_ = failToConvert(ctx, s, id.ItemID, fmeRequestType(id.Type), "%sに失敗しました。%s", fmeRequestType(id.Type).Title(), logmsg)
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

	log.Debugfc(ctx, "featureType: %s", pp.Sprint(featureType))

	// get newitem
	item, err := s.CMS.GetItem(ctx, id.ItemID, false)
	if err != nil {
		log.Errorfc(ctx, "failed to get item: %v", err)
		return fmt.Errorf("failed to get item: %w", err)
	}

	baseFeatureItem := cmsintegrationcommon.FeatureItemFrom(item)

	// get url from the result
	assets := f.GetResultURLs(id.FeatureType)

	// upload assets
	log.Infofc(ctx, "upload assets: %v", assets.Data)
	var dataAssets []string
	dataAssetMap := map[string][]string{}
	if len(assets.DataMap) > 0 {
		dataAssets = make([]string, 0, len(assets.DataMap))
		for _, key := range assets.Keys {
			urls := assets.DataMap[key]
			for _, url := range urls {
				aid, err := s.UploadAsset(ctx, id.ProjectID, url)
				if err != nil {
					log.Errorfc(ctx, "failed to upload asset (%s): %v", url, err)
					return nil
				}
				dataAssets = append(dataAssets, aid)
				dataAssetMap[key] = append(dataAssetMap[key], aid)
			}
		}
	}
	sort.Strings(dataAssets)

	// read dic
	var dic string
	if assets.Dic != "" {
		var err error
		log.Debugfc(ctx, "read and upload dic: %s", assets.Dic)
		dic, err = readDic(ctx, assets.Dic)
		if err != nil {
			log.Errorfc(ctx, "failed to read dic: %v", err)
			return nil
		}
	}

	// upload maxlod
	var maxlodAssetID string
	if assets.MaxLOD != "" {
		log.Debugfc(ctx, "upload maxlod: %s", assets.MaxLOD)
		var err error
		maxlodAssetID, err = s.UploadAsset(ctx, id.ProjectID, assets.MaxLOD)
		if err != nil {
			return fmt.Errorf("failed to upload maxlod: %w", err)
		}
	}

	// upload qc result
	var qcResult string
	if assets.QCResult != "" {
		log.Debugfc(ctx, "upload qc result: %s", assets.QCResult)
		var err error
		qcResult, err = s.UploadAsset(ctx, id.ProjectID, assets.QCResult)
		if err != nil {
			return fmt.Errorf("failed to upload qc result: %w", err)
		}
	}

	// update item
	convStatus := cmsintegrationcommon.ConvertionStatus("")
	qcStatus := cmsintegrationcommon.ConvertionStatus("")

	if id.Type == string(fmeTypeConv) {
		convStatus = cmsintegrationcommon.ConvertionStatusSuccess
	} else if id.Type == string(fmeTypeQC) {
		qcStatus = cmsintegrationcommon.ConvertionStatusSuccess
	} else if id.Type == string(fmeTypeQcConv) {
		convStatus = cmsintegrationcommon.ConvertionStatusSuccess
		qcStatus = cmsintegrationcommon.ConvertionStatusSuccess
	}

	// items
	var data []string
	var items []cmsintegrationcommon.FeatureItemDatum
	if featureType.UseGroups {
		log.Debugfc(ctx, "use groups")

		for _, k := range assets.Keys {
			assets := dataAssetMap[k]
			i, ok := lo.Find(baseFeatureItem.Items, func(i cmsintegrationcommon.FeatureItemDatum) bool {
				return i.Key == k
			})

			var id string
			if ok {
				id = i.ID
			} else {
				id = generateID()
			}

			items = append(items, cmsintegrationcommon.FeatureItemDatum{
				ID:   id,
				Data: assets,
				Key:  k,
			})
		}
	} else {
		data = dataAssets
	}

	newitem := (&cmsintegrationcommon.FeatureItem{
		Data:             data,
		Items:            items,
		Dic:              dic,
		MaxLOD:           maxlodAssetID,
		ConvertionStatus: cmsintegrationcommon.TagFrom(convStatus),
		QCStatus:         cmsintegrationcommon.TagFrom(qcStatus),
		QCResult:         qcResult,
	}).CMSItem()

	log.Debugfc(ctx, "update item: %s", pp.Sprint(newitem))

	newitem2, err := s.CMS.UpdateItem(ctx, id.ItemID, newitem.Fields, newitem.MetadataFields)
	if err != nil {
		j1, _ := json.Marshal(newitem.Fields)
		j2, _ := json.Marshal(newitem.MetadataFields)
		log.Debugfc(ctx, "item update for %s: %s, %s", id.ItemID, j1, j2)
		log.Errorfc(ctx, "failed to update item: %v", err)
		return fmt.Errorf("failed to update item: %w", err)
	}

	log.Debugfc(ctx, "updated item: %s", pp.Sprint(newitem2))

	// comment to the item
	err = s.CMS.CommentToItem(ctx, id.ItemID, fmt.Sprintf("%sが完了しました。%s", fmeRequestType(id.Type).Title(), logmsg))
	if err != nil {
		return fmt.Errorf("failed to add comment: %w", err)
	}

	log.Infofc(ctx, "receiveResultFromFME: success")
	return nil
}

func failToConvert(ctx context.Context, s *Services, itemID string, convType fmeRequestType, message string, args ...any) error {
	if err := s.UpdateFeatureItemStatus(ctx, itemID, convType, cmsintegrationcommon.ConvertionStatusError); err != nil {
		return fmt.Errorf("failed to update item: %w", err)
	}

	if err := s.CMS.CommentToItem(ctx, itemID, fmt.Sprintf(message, args...)); err != nil {
		return fmt.Errorf("failed to add comment: %w", err)
	}

	return nil
}

func readDic(ctx context.Context, u string) (string, error) {
	if u == "" {
		return "", nil
	}

	req, err := http.NewRequestWithContext(ctx, "GET", u, nil)
	if err != nil {
		return "", err
	}
	res, err := http.DefaultClient.Do(req)
	if err != nil {
		return "", err
	}
	defer func() {
		_ = res.Body.Close()
	}()
	if res.StatusCode != http.StatusOK {
		return "", fmt.Errorf("status code is %d", err)
	}
	s, err := io.ReadAll(bom.NewReader(res.Body))
	if err != nil {
		return "", err
	}
	return string(s), nil
}
