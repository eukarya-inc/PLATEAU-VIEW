package citygml

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog"
)

type dataCatalogAPI struct {
	httpClient *http.Client
	url        string
}

func NewDataCatalogAPI(httpClient *http.Client, url string) *dataCatalogAPI {
	if httpClient == nil {
		httpClient = &http.Client{
			Timeout: 10 * time.Second,
		}
	}
	return &dataCatalogAPI{
		httpClient: httpClient,
		url:        url + "/citygml",
	}
}

func (a *dataCatalogAPI) FetchCityGMLFiles(ctx context.Context, cond string) (*datacatalog.CityGMLFilesResponse, error) {
	// Spatial IDs (s:z/f/x/y) contain slashes that must be percent-encoded so the
	// whole condition stays a single path segment; otherwise the datacatalog route
	// (/citygml/:conditions) does not match and returns 404.
	escaped := strings.ReplaceAll(cond, "/", "%2F")
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, a.url+"/"+escaped, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	resp, err := a.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to send request: %w", err)
	}

	defer func() {
		_ = resp.Body.Close()
	}()
	if resp.StatusCode != http.StatusOK {
		if resp.StatusCode == http.StatusNotFound {
			return nil, nil
		}
		return nil, fmt.Errorf("failed to fetch: %d", resp.StatusCode)
	}

	var res datacatalog.CityGMLFilesResponse
	if err := json.NewDecoder(resp.Body).Decode(&res); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	return &res, nil
}
