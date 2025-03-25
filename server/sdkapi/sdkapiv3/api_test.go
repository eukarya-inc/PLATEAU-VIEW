package sdkapiv3

import (
	"context"
	"testing"

	"github.com/k0kubun/pp/v3"
	"github.com/stretchr/testify/assert"
)

func TestQueryDatasets(t *testing.T) {
	baseURL := ""
	gqlToken := ""

	if baseURL == "" {
		t.Skip("baseURL is not set")
	}

	client, err := NewAPIClient(Config{DataCatalogAPIURL: baseURL, DataCatalogAPIToken: gqlToken})
	assert.NoError(t, err)

	q, err := client.QueryDatasets(context.Background())
	assert.NoError(t, err)

	t.Log(pp.Sprint(q))
}

func TestQueryDatasetFiles(t *testing.T) {
	baseURL := ""
	gqlToken := ""
	cityId := ""

	if baseURL == "" {
		t.Skip("baseURL is not set")
	}

	client, err := NewAPIClient(Config{DataCatalogAPIURL: baseURL, DataCatalogAPIToken: gqlToken})
	assert.NoError(t, err)

	q, err := client.QueryDatasetFiles(context.Background(), cityId)
	assert.NoError(t, err)

	t.Log(pp.Sprint(q))
}
