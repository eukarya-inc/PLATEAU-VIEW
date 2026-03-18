package preparegspatialjp

import (
	"context"
	"encoding/csv"
	"fmt"

	"github.com/reearth/reearthx/log"
)

func ValidateLODStat(ctx context.Context, cw *CMSWrapper, mc MergeContext) (err error) {
	if mc.GspatialjpDataItem.MaxLODURL == "" {
		log.Infofc(ctx, "lodstat validation (%s/%s): skipped", mc.CityItem.ID, mc.CityItem.CityName)
		return nil
	}

	// download lodstat csv
	c, err := downloadFile(ctx, mc.GspatialjpDataItem.MaxLODURL)
	if err != nil {
		return fmt.Errorf("failed to download lodstat csv (%s/%s): %w", mc.CityItem.ID, mc.CityItem.CityName, err)
	}

	defer func() {
		_ = c.Close()
	}()

	// validate csv format
	records, err := csv.NewReader(c).ReadAll()
	if err != nil {
		return fmt.Errorf("lodstat validation failed (%s/%s): %w", mc.CityItem.ID, mc.CityItem.CityName, err)
	}

	// Validate header and column count
	// Old format (4 columns): code,type,maxLod,file
	// New format (11 columns): code,type,maxLod,file,filesize,features,lod0,lod1,lod2,lod3,lod4
	if len(records) > 0 {
		numColumns := len(records[0])
		if numColumns != 4 && numColumns != 11 {
			return fmt.Errorf("lodstat validation failed (%s/%s): expected 4 or 11 columns, got %d", mc.CityItem.ID, mc.CityItem.CityName, numColumns)
		}
	}

	log.Infofc(ctx, "lodstat validation (%s/%s): ok (%d records)", mc.CityItem.ID, mc.CityItem.CityName, len(records)-1)
	return nil
}
