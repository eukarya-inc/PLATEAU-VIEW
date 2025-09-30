package preparegspatialjp

import (
	"bufio"
	"bytes"
	"context"
	"encoding/csv"
	"fmt"
	"os"

	"github.com/reearth/reearthx/log"
)

func PrepareLODStat(ctx context.Context, cw *CMSWrapper, mc MergeContext) (err error) {
	defer func() {
		if err == nil {
			return
		}
		err = fmt.Errorf("LOD統計情報のマージに失敗しました: %w", err)
		cw.NotifyError(ctx, err, false, false, true)
	}()

	tmpDir := mc.TmpDir
	cityItem := mc.CityItem
	allFeatureItems := mc.AllFeatureItems

	log.Infofc(ctx, "preparing lodstat...")

	_ = os.MkdirAll(tmpDir, os.ModePerm)

	fileName := fmt.Sprintf("%s_%s_%d_lodstat.csv", cityItem.CityCode, cityItem.CityNameEn, cityItem.YearInt())

	allData := bytes.NewBuffer(nil)

	// Write header for the new format
	// Format: code,type,maxLod,file,filesize,features,lod0,lod1,lod2,lod3,lod4
	if _, err := allData.WriteString("code,type,maxLod,file,filesize,features,lod0,lod1,lod2,lod3,lod4\n"); err != nil {
		return fmt.Errorf("failed to write header: %w", err)
	}

	found := false
	for _, ft := range mc.FeatureTypes {
		fi, ok := allFeatureItems[ft]
		if !ok || fi.MaxLOD == "" {
			log.Infofc(ctx, "no lodstat for %s", ft)
			continue
		}

		log.Infofc(ctx, "downloading lodstat data for %s: %s", ft, fi.MaxLOD)
		data, err := downloadFile(ctx, fi.MaxLOD)
		if err != nil {
			return fmt.Errorf("failed to download data for %s: %w", ft, err)
		}

		b := bufio.NewReader(data)
		// Skip the first line (header)
		if line, err := b.ReadString('\n'); err != nil {
			return fmt.Errorf("failed to read first line: %w", err)
		} else if line == "" || isNumeric(rune(line[0])) {
			// the first line should be header (code,type,maxLod,file,filesize,features,lod0,lod1,lod2,lod3,lod4)
			return fmt.Errorf("invalid lodstat data for %s: missing header", ft)
		}

		if _, err := allData.ReadFrom(b); err != nil {
			return fmt.Errorf("failed to read data for %s: %w", ft, err)
		}

		// if buffer is not ended with \n, add it
		if allData.Len() > 0 {
			if allData.Bytes()[allData.Len()-1] != '\n' {
				allData.WriteByte('\n')
			}
		}

		found = true
	}

	if !found {
		log.Infofc(ctx, "no lodstat data found in the city")
		return nil
	}

	buf := allData.Bytes()

	// validate csv
	records, err := csv.NewReader(bytes.NewReader(buf)).ReadAll()
	if err != nil {
		return fmt.Errorf("invalid lodstat csv data: %w", err)
	}

	// Validate that each record has the expected number of columns (11 for new format, 4 for old format)
	// Old format: code,type,maxLod,file
	// New format: code,type,maxLod,file,filesize,features,lod0,lod1,lod2,lod3,lod4
	if len(records) == 0 {
		return fmt.Errorf("empty lodstat csv data")
	}

	numColumns := len(records[0])
	if numColumns != 4 && numColumns != 11 {
		return fmt.Errorf("invalid header: expected 4 or 11 columns, got %d", numColumns)
	}

	for i, record := range records {
		if i == 0 {
			// Already verified header above
			continue
		}
		if len(record) != numColumns {
			return fmt.Errorf("invalid record at line %d: expected %d columns, got %d", i+1, numColumns, len(record))
		}
	}

	log.Infofc(ctx, "lodstat validation passed: %d records", len(records)-1)

	// upload
	aid, err := cw.UploadNormally(ctx, fileName, bytes.NewReader(buf))
	if err != nil {
		return fmt.Errorf("failed to upload lodstat data: %w", err)
	}

	if err := cw.UpdateDataItem(ctx, &GspatialjpDataItem{
		MergeMaxLODStatus: successTag,
		MaxLOD:            aid,
	}); err != nil {
		return fmt.Errorf("failed to update data item: %w", err)
	}

	log.Infofc(ctx, "lodstat prepared: %s (%d records)", fileName, len(records)-1)
	return nil
}
