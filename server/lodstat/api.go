package lodstat

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"time"

	"github.com/hasura/go-graphql-client"
)

var httpClient = &http.Client{
	Timeout: 30 * time.Second,
}

type APIClient struct {
	client   *graphql.Client
	http     *http.Client
	filesURL string
	token    string
}

func NewAPIClient(conf Config) (*APIClient, error) {
	u, err := url.JoinPath(conf.DataCatalogAPIURL, "/graphql")
	if err != nil {
		return nil, fmt.Errorf("error joining url path: %w", err)
	}

	u2, err := url.JoinPath(conf.DataCatalogAPIURL, "/citygml")
	if err != nil {
		return nil, fmt.Errorf("error joining url path: %w", err)
	}

	c := graphql.NewClient(u, httpClient).WithRequestModifier(func(req *http.Request) {
		if conf.DataCatalogAPIToken != "" {
			req.Header.Set("Authorization", "Bearer "+conf.DataCatalogAPIToken)
		}
	})

	return &APIClient{
		client:   c,
		http:     httpClient,
		filesURL: u2,
		token:    conf.DataCatalogAPIToken,
	}, nil
}

func (c *APIClient) QueryDatasetFilesAll(ctx context.Context, id string) ([]DatasetFilesResponse, error) {
	q := struct {
		Cities []struct {
			Files DatasetFilesResponse `json:"files"`
		} `json:"cities"`
	}{}

	u := fmt.Sprintf("%s/%s", c.filesURL, id)
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, u, nil)
	if err != nil {
		return nil, fmt.Errorf("error creating request: %w", err)
	}

	if c.token != "" {
		req.Header.Set("Authorization", "Bearer "+c.token)
	}

	resp, err := c.http.Do(req)
	if err != nil {
		return nil, fmt.Errorf("error making request: %w", err)
	}

	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode != http.StatusOK {
		if resp.StatusCode == http.StatusNotFound {
			return nil, nil
		}
		return nil, fmt.Errorf("error response: %s", resp.Status)
	}

	if err := json.NewDecoder(resp.Body).Decode(&q); err != nil {
		return nil, fmt.Errorf("error decoding response: %w", err)
	}

	files := make([]DatasetFilesResponse, 0, len(q.Cities))
	for _, city := range q.Cities {
		files = append(files, city.Files)
	}
	return files, nil
}

type DatasetsResponse struct {
	Data []*DatasetPrefectureResponse `json:"data"`
}

type DatasetPrefectureResponse struct {
	ID    string                 `json:"id"`
	Title string                 `json:"title"`
	Data  []*DatasetCityResponse `json:"data"`
}

type DatasetCityResponse struct {
	ID           string   `json:"id"`
	Title        string   `json:"title"`
	Spec         string   `json:"spec"`
	Description  string   `json:"description"`
	FeatureTypes []string `json:"featureTypes"`
}

type DatasetFilesResponse map[string][]DatasetFilesResponseItem

type DatasetFilesResponseItem struct {
	Code     string `json:"code"`
	MaxLod   int    `json:"maxLod"`
	URL      string `json:"url"`
	FileSize int64  `json:"fileSize,omitempty"`
	Features int    `json:"features,omitempty"`
	LOD0     *int   `json:"lod0,omitempty"`
	LOD1     *int   `json:"lod1,omitempty"`
	LOD2     *int   `json:"lod2,omitempty"`
	LOD3     *int   `json:"lod3,omitempty"`
	LOD4     *int   `json:"lod4,omitempty"`
}
