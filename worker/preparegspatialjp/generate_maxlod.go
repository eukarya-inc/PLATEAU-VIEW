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

func PrepareMaxLOD(ctx context.Context, cw *CMSWrapper, mc MergeContext) (err error) {
	defer func() {
		if err == nil {
			return
		}
		err = fmt.Errorf("最大LODのマージに失敗しました: %w", err)
		cw.NotifyError(ctx, err, false, false, true)
	}()

	tmpDir := mc.TmpDir
	cityItem := mc.CityItem
	allFeatureItems := mc.AllFeatureItems

	log.Infofc(ctx, "preparing maxlod...")

	_ = os.MkdirAll(tmpDir, os.ModePerm)

	fileName := fmt.Sprintf("%s_%s_%d_maxlod.csv", cityItem.CityCode, cityItem.CityNameEn, cityItem.YearInt())

	allData := bytes.NewBuffer(nil)

	first := false
	found := false
	for _, ft := range mc.FeatureTypes {
		fi, ok := allFeatureItems[ft]
		if !ok || fi.MaxLOD == "" {
			log.Infofc(ctx, "no maxlod for %s", ft)
			continue
		}

		log.Infofc(ctx, "downloading maxlod data for %s: %s", ft, fi.MaxLOD)
		data, err := downloadFile(ctx, fi.MaxLOD)
		if err != nil {
			return fmt.Errorf("failed to download data for %s: %w", ft, err)
		}

		b := bufio.NewReader(data)
		if first {
			if line, err := b.ReadString('\n'); err != nil { // skip the first line
				return fmt.Errorf("failed to read first line: %w", err)
			} else if line == "" || isNumeric(rune(line[0])) {
				// the first line shold be header (code,type,maxlod,filename)
				return fmt.Errorf("invalid maxlod data for %s", ft)
			}
		} else {
			first = true
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
		log.Infofc(ctx, "no maxlod data found in the city")
		return nil
	}

	buf := allData.Bytes()

	// validate csv
	if _, err := csv.NewReader(bytes.NewReader(buf)).ReadAll(); err != nil {
		return fmt.Errorf("invalid maxlod csv data: %w", err)
	}

	// upload
	aid, err := cw.UploadNormally(ctx, fileName, bytes.NewReader(buf))
	if err != nil {
		return fmt.Errorf("failed to upload maxlod data: %w", err)
	}

	if err := cw.UpdateDataItem(ctx, &GspatialjpDataItem{
		MergeMaxLODStatus: successTag,
		MaxLOD:            aid,
	}); err != nil {
		return fmt.Errorf("failed to update data item: %w", err)
	}

	log.Infofc(ctx, "maxlod prepared: %s", fileName)
	return nil
}
