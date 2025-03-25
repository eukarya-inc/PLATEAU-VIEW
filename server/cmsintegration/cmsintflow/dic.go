package cmsintflow

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"time"

	"github.com/spkg/bom"
)

var httpClient = &http.Client{
	Timeout: 30 * time.Second,
}

func readDic(ctx context.Context, u string) (string, error) {
	if u == "" {
		return "", nil
	}

	req, err := http.NewRequestWithContext(ctx, "GET", u, nil)
	if err != nil {
		return "", err
	}

	res, err := httpClient.Do(req)
	if err != nil {
		return "", err
	}
	defer func() {
		_ = res.Body.Close()
	}()
	if res.StatusCode != http.StatusOK {
		return "", fmt.Errorf("unexpected status code %d for URL %s", res.StatusCode, u)
	}
	s, err := io.ReadAll(bom.NewReader(res.Body))
	if err != nil {
		return "", err
	}
	return string(s), nil
}
