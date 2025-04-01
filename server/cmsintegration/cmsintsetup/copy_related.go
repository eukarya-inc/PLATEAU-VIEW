package cmsintsetup

import (
	"context"
	"fmt"
	"strings"

	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/log"
)

type CopyRelatedItemsOpts struct {
	FromProject   string `json:"fromProject"`
	ToProject     string `json:"toProject"`
	Offset        int    `json:"offset"`
	Limit         int    `json:"limit"`
	DryRun        bool   `json:"dryRun"`
	Force         bool   `json:"force"`
	CacheBasePath string `json:"-"`
}

func CopyRelatedDatasetItems(ctx context.Context, s *Services, opts CopyRelatedItemsOpts) error {
	if opts.FromProject == "" || opts.ToProject == "" {
		return fmt.Errorf("fromProject and toProject are required")
	}

	pcms := s.PlateauCMS
	if pcms == nil {
		pcms = NewPlateauCMS(s.CMS, opts.CacheBasePath)
	}

	log.Infofc(ctx, "cmsintegrationv3: copy_related: copy related dataset items from %s to %s", opts.FromProject, opts.ToProject)
	if opts.DryRun {
		log.Infofc(ctx, "cmsintegrationv3: copy_related: DRY RUN")
	}

	log.Infofc(ctx, "cmsintegrationv3: copy_related: loading items from CMS...")

	// get relatedDataTypes
	datasetTypes, err := s.PCMS.DatasetTypes(ctx)
	if err != nil {
		return fmt.Errorf("failed to get dataset types: %w", err)
	}

	relatedDataTypes := datasetTypes.Codes(plateaucms.DatasetCategoryRelated)

	// get model info
	model1, err := s.CMS.GetModelByKey(ctx, opts.FromProject, cmsintegrationcommon.ModelPrefix+cmsintegrationcommon.RelatedModel)
	if err != nil || model1 == nil {
		return fmt.Errorf("failed to get model from %s: %w", opts.FromProject, err)
	}

	model2, err := s.CMS.GetModelByKey(ctx, opts.ToProject, cmsintegrationcommon.ModelPrefix+cmsintegrationcommon.RelatedModel)
	if err != nil || model2 == nil {
		return fmt.Errorf("failed to get model from %s: %w", opts.ToProject, err)
	}

	// get all cities and create a map of city item IDs between two projects
	citiesSrc, err := pcms.GetAllCities(ctx, opts.FromProject, nil)
	if err != nil {
		return fmt.Errorf("failed to get cities from %s: %w", opts.FromProject, err)
	}
	citiesTarget, err := pcms.GetAllCities(ctx, opts.ToProject, nil)
	if err != nil {
		return fmt.Errorf("failed to get cities from %s: %w", opts.ToProject, err)
	}

	citySrcMap := map[string]*cmsintegrationcommon.CityItem{}
	cityTargetMap := map[string]*cmsintegrationcommon.CityItem{}
	for _, city := range citiesSrc {
		citySrcMap[city.ID] = city
	}
	for _, city := range citiesTarget {
		cityTargetMap[city.ID] = city
	}

	// get all related items
	relatedsSrc, err := pcms.GetAllRelated(ctx, opts.FromProject, relatedDataTypes)
	if err != nil {
		return fmt.Errorf("failed to get related items from %s: %w", opts.FromProject, err)
	}
	relatedTarget, err := pcms.GetAllRelated(ctx, opts.ToProject, relatedDataTypes)
	if err != nil {
		return fmt.Errorf("failed to get related items from %s: %w", opts.ToProject, err)
	}

	cityCodeToRelatedTarget := map[string]*cmsintegrationcommon.RelatedItem{}
	for _, related := range relatedTarget {
		city := cityTargetMap[related.City]
		if city == nil {
			continue
		}

		cityCodeToRelatedTarget[city.CityCode] = related
	}

	// cut list by offset and limit
	if opts.Offset > 0 || opts.Limit > 0 {
		relatedsSrc = relatedsSrc[opts.Offset : opts.Limit+opts.Offset]
	}

	log.Infofc(ctx, "cmsintegrationv3: copy_related: copy %d related items", len(relatedsSrc))
	skipped := 0
	notFound := []string{}
	alreadyExists := []string{}

	// update related items in the target project
	for i, r := range relatedsSrc {
		if r.City == "" || len(r.Items) == 0 {
			skipped++
			continue
		}

		srcCity, ok := citySrcMap[r.City]
		if !ok {
			log.Errorfc(ctx, "cmsintegrationv3: copy_related: city item not found in source project: %s", r.City)
			continue
		}

		targetRelated := cityCodeToRelatedTarget[srcCity.CityCode]
		if targetRelated == nil {
			notFound = append(notFound, srcCity.CityCode+" "+srcCity.CityName)
			continue
		}

		if !opts.Force && len(targetRelated.Items) > 0 {
			alreadyExists = append(alreadyExists, srcCity.CityCode+" "+srcCity.CityName)
			continue
		}

		newRelatedItem := *r
		newRelatedItem.ID = targetRelated.ID
		newRelatedItem.City = targetRelated.City
		if newRelatedItem.Status != nil {
			newRelatedItem.Status = &cms.Tag{Name: newRelatedItem.Status.Name}
		}
		if newRelatedItem.MergeStatus != nil {
			newRelatedItem.MergeStatus = &cms.Tag{Name: newRelatedItem.MergeStatus.Name}
		}
		for k, tag := range newRelatedItem.ConvertStatus {
			if tag != nil {
				newRelatedItem.ConvertStatus[k] = &cms.Tag{Name: tag.Name}
			}
		}
		newItem := newRelatedItem.CMSItem(relatedDataTypes)

		// update a related item
		if !opts.DryRun {
			_, err := s.CMS.UpdateItem(ctx, newItem.ID, newItem.Fields, newItem.MetadataFields)
			if err != nil {
				return fmt.Errorf("failed to create related item: %w", err)
			}
		}

		log.Infofc(ctx, "cmsintegrationv3: copy_related: ok (%d): %s %s %s", i+1, newItem.ID, srcCity.CityCode, srcCity.CityName)
	}

	log.Infofc(ctx, "cmsintegrationv3: copy_related: done")

	if skipped > 0 {
		log.Infofc(ctx, "cmsintegrationv3: copy_related: skipped %d related items", skipped)
	}
	if len(notFound) > 0 {
		log.Warnfc(ctx, "cmsintegrationv3: copy_related: city items not found in target project: \n%v", strings.Join(notFound, "\n"))
	}
	if len(alreadyExists) > 0 {
		log.Warnfc(ctx, "cmsintegrationv3: copy_related: related items already exist in target project: \n%v", strings.Join(alreadyExists, "\n"))
	}

	return nil
}
