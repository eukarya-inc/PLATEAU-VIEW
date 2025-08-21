package sdkapiv3

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"

	"github.com/hasura/go-graphql-client"
)

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

	hc := http.DefaultClient
	c := graphql.NewClient(u, hc).WithRequestModifier(func(req *http.Request) {
		if conf.DataCatalogAPIToken != "" {
			req.Header.Set("Authorization", "Bearer "+conf.DataCatalogAPIToken)
		}
	})

	return &APIClient{
		client:   c,
		http:     hc,
		filesURL: u2,
		token:    conf.DataCatalogAPIToken,
	}, nil
}

func (c *APIClient) QueryDatasets(ctx context.Context) (DatasetsQuery, error) {
	var q DatasetsQuery

	err := c.client.Query(ctx, &q, nil)
	if err != nil {
		return q, fmt.Errorf("error querying datasets: %w", err)
	}

	return q, nil
}

func (c *APIClient) QueryDatasetFiles(ctx context.Context, id string) (DatasetFilesResponse, error) {
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

	return q.Cities[0].Files, nil
}
