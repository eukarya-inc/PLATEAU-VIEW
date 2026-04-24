package datacatalog

import (
	"compress/gzip"
	"context"
	"encoding/csv"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"path"
	"slices"
	"strconv"
	"strings"
	"time"

	"github.com/reearth/reearthx/rerror"
	"github.com/samber/lo"
	"github.com/spkg/bom"
	"golang.org/x/sync/errgroup"
)

var httpClient = &http.Client{
	Timeout: 30 * time.Second,
}

func fetchCSVs(ctx context.Context, urls, citygmlBaseURLs []string) (records [][]string, _ error) {
	if len(urls) != len(citygmlBaseURLs) {
		return nil, fmt.Errorf("length of urls and citygmlBaseURLs must be the same")
	}

	results := make([][][]string, len(urls))
	errg := errgroup.Group{}
	errg.SetLimit(10)

	for i, url := range urls {
		base := citygmlBaseURLs[i]
		errg.Go(func() error {
			data, err := fetchCSV(ctx, url, base)
			if err != nil {
				return fmt.Errorf("failed to fetch %s: %w", url, err)
			}

			results[i] = data
			return nil
		})
	}

	if err := errg.Wait(); err != nil {
		return nil, err
	}

	// Merge results after all goroutines complete
	for _, data := range results {
		records = append(records, data...)
	}

	return records, nil
}

func fetchCSV(ctx context.Context, url, prefix string) (records [][]string, _ error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	// Request gzip encoding for faster downloads
	req.Header.Set("Accept-Encoding", "gzip")

	resp, err := httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to request: %w", err)
	}

	defer func() {
		_ = resp.Body.Close()
	}()

	if resp.StatusCode != http.StatusOK {
		if resp.StatusCode == http.StatusNotFound {
			return nil, rerror.ErrNotFound
		}
		return nil, fmt.Errorf("failed to request: %w", err)
	}

	// Handle gzip decompression if content is gzip encoded
	var reader io.Reader = resp.Body
	if resp.Header.Get("Content-Encoding") == "gzip" {
		gzipReader, err := gzip.NewReader(resp.Body)
		if err != nil {
			return nil, fmt.Errorf("failed to create gzip reader: %w", err)
		}
		defer func() {
			_ = gzipReader.Close()
		}()
		reader = gzipReader
	}

	c := csv.NewReader(bom.NewReader(reader))
	for {
		record, err := c.Read()
		if err == io.EOF {
			break
		}
		if err != nil {
			return nil, fmt.Errorf("failed to read csv: %w", err)
		}

		record = append([]string{prefix}, record...)
		records = append(records, record)
	}

	return
}

func csvToCityGMLFilesResponse(data [][]string, gmlURLs []*url.URL) CityGMLFiles {
	res := make(CityGMLFiles)

	for _, record := range data {
		if len(record) < 3 || record[0] == "" || record[1] == "" {
			continue
		}

		// Strip Windows-style backslash directory prefix that may leak into
		// `code` and `file` columns when a lodstat CSV is generated from a ZIP
		// whose entries use backslash separators.
		if i := strings.LastIndex(record[1], `\`); i >= 0 {
			record[1] = record[1][i+1:]
		}
		if len(record) > 4 {
			if i := strings.LastIndex(record[4], `\`); i >= 0 {
				record[4] = record[4][i+1:]
			}
		}

		if record[1] == "" || !isNumeric(rune(record[1][0])) {
			continue // skip header
		}

		const clen = 12
		if len(record) < clen {
			// expand record with empty values
			record = append(record, make([]string, clen-len(record))...)
		}

		// base,code,type,maxLod,file
		// base,code,type,maxLod,file,filesize,features,lod0,lod1,lod2,lod3,lod4
		base := record[0]
		meshCode := record[1]                              // code
		featureType := record[2]                           // type
		maxlod, _ := strconv.Atoi(record[3])               // maxLod
		gmlPath := record[4]                               // file
		fileSize, _ := strconv.ParseInt(record[5], 10, 64) // filesize
		features, _ := strconv.Atoi(record[6])             // features
		lod0 := parseLOD(record[7])
		lod1 := parseLOD(record[8])
		lod2 := parseLOD(record[9])
		lod3 := parseLOD(record[10])
		lod4 := parseLOD(record[11])

		citygmlURL := ""
		if len(record) > 4 && gmlURLs == nil {
			citygmlURL = citygmlItemURLFrom(base, gmlPath, featureType)
		} else {
			// compat for datacatalogv2
			prefix := fmt.Sprintf("%s_%s_", meshCode, featureType)

			u, ok := lo.Find(gmlURLs, func(u *url.URL) bool {
				return strings.HasPrefix(path.Base(u.Path), prefix) && path.Ext(u.Path) == ".gml"
			})
			if ok {
				citygmlURL = u.String()
			}
			// warning = append(warning, fmt.Sprintf("unmatched:type=%s,code=%s,path=%s", ty, code, f))
		}

		if citygmlURL == "" {
			continue
		}

		item := CityGMLFile{
			MeshCode: meshCode,
			MaxLOD:   maxlod,
			URL:      citygmlURL,
			FileSize: lo.EmptyableToPtr(fileSize),
			Features: lo.EmptyableToPtr(features),
			LOD0:     lod0,
			LOD1:     lod1,
			LOD2:     lod2,
			LOD3:     lod3,
			LOD4:     lod4,
		}

		if _, ok := res[featureType]; !ok {
			res[featureType] = make([]CityGMLFile, 0)
		}

		res[featureType] = append(res[featureType], item)
	}

	for _, v := range res {
		slices.SortFunc(v, func(i, j CityGMLFile) int {
			return strings.Compare(i.MeshCode, j.MeshCode)
		})
	}

	return res
}
