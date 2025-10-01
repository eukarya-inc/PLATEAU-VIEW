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
	reader := csv.NewReader(bytes.NewReader(buf))
	reader.FieldsPerRecord = -1 // Allow variable number of fields
	records, err := reader.ReadAll()
	if err != nil {
		return fmt.Errorf("invalid lodstat csv data: %w", err)
	}

	if len(records) == 0 {
		return fmt.Errorf("empty lodstat csv data")
	}

	// Find the maximum number of columns
	maxColumns := 0
	for _, record := range records {
		if len(record) > maxColumns {
			maxColumns = len(record)
		}
	}

	// Validate that maxColumns is either 4 or 11
	// Old format: code,type,maxLod,file
	// New format: code,type,maxLod,file,filesize,features,lod0,lod1,lod2,lod3,lod4
	if maxColumns != 4 && maxColumns != 11 {
		return fmt.Errorf("invalid header: expected 4 or 11 columns, got %d", maxColumns)
	}

	// Normalize records to have the same number of columns
	for i, record := range records {
		if len(record) < maxColumns {
			// Pad with empty strings
			padded := make([]string, maxColumns)
			copy(padded, record)
			for j := len(record); j < maxColumns; j++ {
				padded[j] = ""
			}
			records[i] = padded
		}
	}

	log.Infofc(ctx, "lodstat validation passed: %d records with %d columns", len(records)-1, maxColumns)

	// Regenerate CSV with normalized records
	normalizedBuf := bytes.NewBuffer(nil)
	writer := csv.NewWriter(normalizedBuf)
	if err := writer.WriteAll(records); err != nil {
		return fmt.Errorf("failed to write normalized csv: %w", err)
	}
	writer.Flush()
	if err := writer.Error(); err != nil {
		return fmt.Errorf("failed to flush csv writer: %w", err)
	}
	buf = normalizedBuf.Bytes()

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
