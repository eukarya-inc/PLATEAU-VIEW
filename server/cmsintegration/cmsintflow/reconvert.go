package cmsintflow

import (
	"context"
	"errors"
	"fmt"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/log"
	"github.com/samber/lo"
)

// ReconvertRequest asks to run the conversion of a feature item again with the Flow trigger
// configured for its spec, without QC. It is used to add the JGD2024 3D Tiles to items that
// were converted before: the converted assets are merged into the item by CRS, so the
// existing JGD2011 assets are kept.
type ReconvertRequest struct {
	ProjectID string `json:"projectId"`
	// Model is the key of the feature model, such as "plateau-bldg".
	Model  string `json:"model"`
	ItemID string `json:"itemId"`
}

type ReconvertResult struct {
	ItemID string `json:"itemId"`
	// Started reports whether the conversion was requested to Flow.
	Started bool `json:"started"`
	// ConvStatus is conv_status of the item after the request.
	ConvStatus string `json:"convStatus"`
	// Message is why the conversion was not started.
	Message string `json:"message,omitempty"`
}

// ErrReconvertRunning is returned when QC or conversion of the item is running.
var ErrReconvertRunning = errors.New("qc or conversion is running")

// ErrReconvertNotSupported is returned when the item is not converted with Flow.
var ErrReconvertNotSupported = errors.New("the item is not converted with flow")

// ErrReconvertInvalidRequest is returned when the request lacks a field.
var ErrReconvertInvalidRequest = errors.New("projectId, itemId and model (" + cmsintegrationcommon.ModelPrefix + "*) are required")

// reconvert starts the conversion of the item. Starting a conversion through the webhook would
// need its status to be changed by someone other than this server's integration, since the
// webhook ignores the events the server causes itself, and would also need skip_qc_conv to be
// rewritten to skip QC. This calls the same request path directly instead.
func reconvert(ctx context.Context, s *Services, conf *Config, req ReconvertRequest) (*ReconvertResult, error) {
	if req.ProjectID == "" || req.ItemID == "" || !strings.HasPrefix(req.Model, cmsintegrationcommon.ModelPrefix) {
		return nil, ErrReconvertInvalidRequest
	}

	item, err := s.CMS.GetItem(ctx, req.ItemID, false)
	if err != nil {
		return nil, fmt.Errorf("failed to get item: %w", err)
	}
	mainItem, err := s.itemWithMetadata(ctx, item)
	if err != nil {
		return nil, err
	}

	fi := cmsintegrationcommon.FeatureItemFrom(mainItem)
	if isRunning(fi.QCStatus) || isRunning(fi.ConvertionStatus) {
		return nil, ErrReconvertRunning
	}

	featureTypes, err := s.PCMS.PlateauFeatureTypes(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get feature types: %w", err)
	}
	specs, err := s.PCMS.PlateauSpecs(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get plateau specs: %w", err)
	}

	// sendRequestToFlow sets conv_status to 実行中 before it checks the converter, and then
	// returns without requesting anything when the item does not use Flow, which would leave
	// the item 実行中 forever. So the same checks are made here first.
	if err := checkFlowConversion(ctx, s, req.Model, fi, featureTypes, plateaucms.PlateauSpecList(specs)); err != nil {
		return nil, err
	}

	// The workflow starts a conversion only from conv_status 未実行 (or empty), so a converted
	// item is reset first. The previous value is restored when no conversion is started.
	prevConv := fi.ConvertionStatus
	if !isNotStarted(prevConv) {
		if err := s.UpdateStatus(ctx, mainItem.ID, "", cmsintegrationcommon.ConvertionStatusNotStarted); err != nil {
			return nil, fmt.Errorf("failed to reset conv_status: %w", err)
		}
		setMetadataTag(mainItem, "conv_status", string(cmsintegrationcommon.ConvertionStatusNotStarted))
	}

	modelName := strings.TrimPrefix(req.Model, cmsintegrationcommon.ModelPrefix)
	reqErr := sendRequestToFlow(ctx, s, conf, req.ProjectID, modelName, mainItem, featureTypes, plateaucms.PlateauSpecList(specs), cmsintegrationcommon.ReqTypeConv)

	status, err := s.convStatus(ctx, mainItem)
	if err != nil {
		return nil, errors.Join(reqErr, err)
	}

	res := &ReconvertResult{
		ItemID:     mainItem.ID,
		Started:    status == string(cmsintegrationcommon.ConvertionStatusRunning),
		ConvStatus: status,
	}
	if reqErr != nil {
		res.Message = reqErr.Error()
	}

	// When no conversion was started, put conv_status back, so that an item whose JGD2011 data
	// is published does not show 未実行, or エラー for a request that never reached Flow. The
	// comment sendRequestToFlow posts on failure is kept, which tells why.
	if !res.Started && prevConv != nil && prevConv.Name != status {
		if err := s.UpdateStatus(ctx, mainItem.ID, "", cmsintegrationcommon.ConvertionStatus(prevConv.Name)); err != nil {
			log.Warnfc(ctx, "reconvert: failed to restore conv_status of %s: %v", mainItem.ID, err)
		} else {
			res.ConvStatus = prevConv.Name
		}
	}

	return res, reqErr
}

// itemWithMetadata returns the item with its metadata fields. The integration API returns them
// in metadataFields of the item. Webhook payloads instead carry the main and the metadata item
// separately, linked by their IDs.
func (s *Services) itemWithMetadata(ctx context.Context, item *cms.Item) (*cms.Item, error) {
	if item.MetadataItemID == nil && item.OriginalItemID == nil {
		return item, nil
	}
	return s.GetMainItemWithMetadata(ctx, item)
}

func (s *Services) convStatus(ctx context.Context, mainItem *cms.Item) (string, error) {
	item, err := s.CMS.GetItem(ctx, mainItem.ID, false)
	if err != nil {
		return "", fmt.Errorf("failed to get item: %w", err)
	}
	if item, err = s.itemWithMetadata(ctx, item); err != nil {
		return "", err
	}
	if t := item.MetadataFieldByKey("conv_status").GetValue().Tag(); t != nil {
		return t.Name, nil
	}
	return "", nil
}

func isRunning(t *cms.Tag) bool {
	return t != nil && t.Name == string(cmsintegrationcommon.ConvertionStatusRunning)
}

func isNotStarted(t *cms.Tag) bool {
	return t == nil || t.Name == "" || t.Name == string(cmsintegrationcommon.ConvertionStatusNotStarted)
}

// setMetadataTag sets the value of a tag field of the metadata in the shape the CMS API returns.
func setMetadataTag(item *cms.Item, key, name string) {
	for _, f := range item.MetadataFields {
		if f != nil && f.Key == key {
			f.Value = map[string]any{"name": name}
			return
		}
	}
	item.MetadataFields = append(item.MetadataFields, &cms.Field{Key: key, Type: "tag", Value: map[string]any{"name": name}})
}

// checkFlowConversion reports why the conversion of the item would not be requested to Flow.
func checkFlowConversion(ctx context.Context, s *Services, model string, fi *cmsintegrationcommon.FeatureItem, featureTypes plateaucms.PlateauFeatureTypeList, specs plateaucms.PlateauSpecList) error {
	if fi.City == "" {
		return fmt.Errorf("%w: no city", ErrReconvertNotSupported)
	}
	cityRaw, err := s.CMS.GetItem(ctx, fi.City, false)
	if err != nil {
		return fmt.Errorf("failed to get city item: %w", err)
	}
	city := cmsintegrationcommon.CityItemFrom(cityRaw, featureTypes.Codes())
	if err := FlowConversionError(model, fi, city, featureTypes, specs); err != nil {
		return fmt.Errorf("%w: %w", ErrReconvertNotSupported, err)
	}
	return nil
}

// FlowConversionError reports why the conversion of the feature item would not be requested
// to Flow, following the same rules as sendRequestToFlow. It returns nil when it would be.
func FlowConversionError(model string, fi *cmsintegrationcommon.FeatureItem, city *cmsintegrationcommon.CityItem, featureTypes plateaucms.PlateauFeatureTypeList, specs plateaucms.PlateauSpecList) error {
	if fi.CityGML == "" || fi.City == "" {
		return errors.New("no city or no citygml")
	}

	modelName := strings.TrimPrefix(model, cmsintegrationcommon.ModelPrefix)
	baseModelName := cmsintegrationcommon.ExtractBaseFeatureType(modelName)
	ftCode := fi.FeatureTypeCode()
	baseFTCode := cmsintegrationcommon.ExtractBaseFeatureType(ftCode)
	ft, ok := lo.Find(featureTypes, func(ft plateaucms.PlateauFeatureType) bool {
		return ft.Code == modelName || ft.Code == baseModelName || ft.Code == ftCode || ft.Code == baseFTCode
	})
	if !ok || !ft.Conv {
		return fmt.Errorf("feature type of %s has no conversion", model)
	}

	specv := fi.SpecMajorVersionInt()
	if specv == 0 {
		specv = city.SpecMajorVersionInt()
	}
	spec := specs.FindByVersion(specv)
	if spec == nil {
		return fmt.Errorf("no plateau spec for version %d", specv)
	}

	converter := cmsintegrationcommon.GetEffectiveConverter(fi, city, spec.Converter)
	if !plateaucms.IsFlowEnabledConverter(converter) {
		return fmt.Errorf("converter is %q", converter)
	}
	if converter == plateaucms.ConverterFMEFlow && !spec.ShouldUseFlow(ft.Code) {
		return fmt.Errorf("feature type %s does not use flow in spec %d", ft.Code, specv)
	}
	if spec.GetFlowConvTrigger(ft.Code) == "" {
		return fmt.Errorf("no flow conversion trigger for %s in spec %d", ft.Code, specv)
	}
	return nil
}
