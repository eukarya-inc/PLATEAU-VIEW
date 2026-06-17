package geocoding

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net"
	"net/http"
	"strconv"
)

// ErrGSIUnavailable indicates that the GSI API is temporarily unavailable (502/503/504)
var ErrGSIUnavailable = errors.New("GSI API is temporarily unavailable")

const DefaultGSIURL = "https://mreversegeocoder.gsi.go.jp/reverse-geocoder/LonLatToAddress"

type GSIClient struct {
	httpClient *http.Client
	url        string
}

type GSIResult struct {
	MunicipalityCode string
	Name             string
}

type gsiResponse struct {
	Results *gsiResults `json:"results"`
}

type gsiResults struct {
	MuniCd string `json:"muniCd"`
	Lv01Nm string `json:"lv01Nm"`
}

func NewGSIClient(httpClient *http.Client, url string) *GSIClient {
	if httpClient == nil {
		httpClient = http.DefaultClient
	}
	if url == "" {
		url = DefaultGSIURL
	}
	return &GSIClient{
		httpClient: httpClient,
		url:        url,
	}
}

func (c *GSIClient) Fetch(ctx context.Context, lon, lat float64) (*GSIResult, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, c.url, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	q := req.URL.Query()
	q.Set("lon", strconv.FormatFloat(lon, 'f', -1, 64))
	q.Set("lat", strconv.FormatFloat(lat, 'f', -1, 64))
	req.URL.RawQuery = q.Encode()

	resp, err := c.httpClient.Do(req)
	if err != nil {
		// A timeout (e.g. the client-side http.Client.Timeout firing on a
		// hung-but-alive upstream) means GSI is effectively unavailable, so
		// surface it as ErrGSIUnavailable to trigger the Nominatim fallback.
		// The request-context cancellation is filtered out separately by the
		// handler before it inspects this error.
		if isTimeoutErr(err) {
			return nil, fmt.Errorf("%w: %v", ErrGSIUnavailable, err)
		}
		return nil, fmt.Errorf("failed to send request: %w", err)
	}
	defer func() {
		_ = resp.Body.Close()
	}()

	if resp.StatusCode != http.StatusOK {
		// Return specific error for upstream service unavailability
		if resp.StatusCode == http.StatusBadGateway ||
			resp.StatusCode == http.StatusServiceUnavailable ||
			resp.StatusCode == http.StatusGatewayTimeout {
			return nil, fmt.Errorf("%w: status code %d", ErrGSIUnavailable, resp.StatusCode)
		}
		return nil, fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	var gsiResp gsiResponse
	if err := json.NewDecoder(resp.Body).Decode(&gsiResp); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	if gsiResp.Results == nil {
		return nil, nil
	}

	return &GSIResult{
		MunicipalityCode: gsiResp.Results.MuniCd,
		Name:             gsiResp.Results.Lv01Nm,
	}, nil
}

// isTimeoutErr reports whether err is a request timeout, covering both the
// http.Client.Timeout deadline and lower-level network timeouts.
func isTimeoutErr(err error) bool {
	if errors.Is(err, context.DeadlineExceeded) {
		return true
	}
	var netErr net.Error
	return errors.As(err, &netErr) && netErr.Timeout()
}
