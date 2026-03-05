package geospatialjpv3

import (
	"fmt"
	"net/http"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/ckan"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/k0kubun/pp/v3"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/log"
	"github.com/samber/lo"
)

const (
	modelKey = "plateau-city"
)

func WebhookHandler(conf Config) (cmswebhook.Handler, error) {
	c, err := cms.New(conf.CMSBase, conf.CMSToken)
	if err != nil {
		return nil, fmt.Errorf("failed to init cms: %w", err)
	}

	ck, err := ckan.New(conf.CkanBase, conf.CkanToken)
	if err != nil {
		return nil, fmt.Errorf("failed to init ckan: %w", err)
	}

	pcms, err := plateaucms.New(plateaucms.Config{
		CMSBaseURL:       conf.CMSBase,
		CMSMainToken:     conf.CMSToken,
		CMSSystemProject: conf.CMSSystemProject,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to init plateau cms: %w", err)
	}

	return (&handler{
		cms:      c,
		ckan:     ck,
		ckanOrg:  conf.CkanOrg,
		ckanBase: conf.CkanBase,
		pcms:     pcms,
	}).Webhook(conf)
}

type handler struct {
	cms      cms.Interface
	ckan     ckan.Interface
	ckanOrg  string
	ckanBase string
	pcms     plateaucms.FeatureTypeStore
}

const prepareFieldKey = "geospatialjp_prepare"
const publishFieldKey = "geospatialjp_publish"

func (h *handler) Webhook(conf Config) (cmswebhook.Handler, error) {
	return func(req *http.Request, w *cmswebhook.Payload) error {
		if req == nil || w == nil {
			log.Debugf("geospatialjpv3 webhook: invalid payload")
			return nil
		}

		ctx := req.Context()
		ctx = log.WithPrefixMessage(ctx, "geospatialjpv3 webhook: ")

		if !w.Operator.IsUser() && w.Operator.IsIntegrationBy(conf.CMSIntegration) {
			log.Debugfc(ctx, "invalid event operator: %+v", w.Operator)
			return nil
		}

		if w.Type != cmswebhook.EventItemUpdate {
			log.Debugfc(ctx, "invalid event type: %s", w.Type)
			return nil
		}

		if w.ItemData == nil || w.ItemData.Item == nil || w.ItemData.Model == nil {
			log.Debugfc(ctx, "invalid event data: %+v", w.Data)
			return nil
		}

		if w.ItemData.Model.Key != modelKey {
			log.Debugfc(ctx, "invalid model id: %s, key: %s", w.ItemData.Item.ModelID, w.ItemData.Model.Key)
			return nil
		}

		if !w.ItemData.Item.IsMetadata {
			log.Debugfc(ctx, "not metadata item")
			return nil
		}

		item, err := GetMainItemWithMetadata(ctx, h.cms, w.ItemData.Item)
		if err != nil {
			log.Errorfc(ctx, "failed to get main item: %v", err)
			return nil
		}

		// feature types
		featureTypes, err := h.pcms.PlateauFeatureTypes(ctx)
		if err != nil {
			log.Errorfc(ctx, "failed to get feature types: %v", err)
			return nil
		}
		featureTypeCodes := featureTypes.Codes()
		featureTypeNames := featureTypes.CodeNameMap()

		cityItem := CityItemFrom(item, featureTypeCodes)

		if cityItem.ID == "" {
			log.Debugfc(ctx, "invalid city item id")
			return nil
		}

		if cityItem.CityCode == "" || cityItem.CityName == "" || cityItem.CityNameEn == "" {
			log.Debugfc(ctx, "invalid city item")

			comment := "この都市は都市名、都市コード、都市英名が入力されてないため、G空間情報センター公開に関する処理が行えません。"
			if h.cms.CommentToItem(ctx, cityItem.ID, comment) != nil {
				log.Errorfc(ctx, "failed to comment to city item: %v", err)
			}
			return nil
		}

		if cityItem.GeospatialjpData == "" || cityItem.GeospatialjpIndex == "" {
			log.Debugfc(ctx, "no data and index id in city")

			comment := "この都市はG空間情報センターに関するアイテムが正しくリンクされていないため、G空間情報センター公開に関する処理が行えません。"
			if h.cms.CommentToItem(ctx, cityItem.ID, comment) != nil {
				log.Errorfc(ctx, "failed to comment to city item: %v", err)
			}
			return nil
		}

		if cityItem.SpecVersionMajorInt() == 0 {
			comment := "この都市は仕様書バージョンが正しく設定されていないため、G空間情報センター公開に関する処理が行えません。"
			if h.cms.CommentToItem(ctx, cityItem.ID, comment) != nil {
				log.Errorfc(ctx, "failed to comment to city item: %v", err)
			}
			return nil
		}

		if cityItem.YearInt() == 0 {
			comment := "この都市は整備年度が正しく設定されていないため、G空間情報センター公開に関する処理が行えません。"
			if h.cms.CommentToItem(ctx, cityItem.ID, comment) != nil {
				log.Errorfc(ctx, "failed to comment to city item: %v", err)
			}
			return nil
		}

		log.Debugfc(ctx, "%s", pp.Sprint(cityItem))

		if b := getChangedBool(w, prepareFieldKey); b != nil && *b {
			if err := Prepare(ctx, cityItem.ID, w.ProjectID(), conf, featureTypeCodes, featureTypeNames); err != nil {
				log.Errorfc(ctx, "failed to prepare: %v", err)
			}
		} else {
			log.Debugfc(ctx, "prepare field not changed or not true")
		}

		if b := getChangedBool(w, publishFieldKey); b != nil && *b {
			if err := h.Publish(ctx, cityItem); err != nil {
				log.Errorfc(ctx, "failed to publish: %v", err)
			}
		} else if b != nil && !*b {
			if err := h.Unpublish(ctx, cityItem); err != nil {
				log.Errorfc(ctx, "failed to unpublish: %v", err)
			}
		} else {
			log.Debugfc(ctx, "publish field not changed or not true")
		}

		log.Debugfc(ctx, "done")
		return nil
	}, nil
}

func getChangedBool(w *cmswebhook.Payload, key string) *bool {
	// w.ItemData.Item is a metadata item, so we need to use FieldByKey instead of MetadataFieldByKey
	if f := w.ItemData.Item.FieldByKey(key); f != nil {
		changed, ok := lo.Find(w.ItemData.Changes, func(c cms.FieldChange) bool {
			return c.ID == f.ID
		})

		if ok {
			b := changed.GetCurrentValue().Bool()
			if b != nil {
				return b
			}

			// workaround for bool array: value is [true]
			if res := changed.GetCurrentValue().Bools(); len(res) > 0 {
				return lo.ToPtr(res[0])
			}
		}
	}

	return nil
}
