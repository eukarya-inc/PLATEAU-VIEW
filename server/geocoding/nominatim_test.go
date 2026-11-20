package geocoding

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestNominatimClient_Fetch(t *testing.T) {
	tests := []struct {
		name           string
		responseBody   string
		responseStatus int
		wantCode       string
		wantErr        bool
	}{
		{
			name: "success - city",
			responseBody: `{
				"display_name": "不忍通り, 目白台二丁目, 目白台, 文京区, 東京都, 171-0032, 日本",
				"address": {
					"city": "文京区",
					"ISO3166-2-lvl4": "JP-13",
					"country_code": "jp"
				}
			}`,
			responseStatus: http.StatusOK,
			wantCode:       "13105",
			wantErr:        false,
		},
		{
			name: "success - town",
			responseBody: `{
				"display_name": "Test Address",
				"address": {
					"town": "千代田区",
					"ISO3166-2-lvl4": "JP-13",
					"country_code": "jp"
				}
			}`,
			responseStatus: http.StatusOK,
			wantCode:       "13101",
			wantErr:        false,
		},
		{
			name: "no address",
			responseBody: `{
				"display_name": "Test Address",
				"address": null
			}`,
			responseStatus: http.StatusOK,
			wantCode:       "",
			wantErr:        false,
		},
		{
			name: "non-japan address",
			responseBody: `{
				"display_name": "Test Address",
				"address": {
					"city": "New York",
					"country_code": "us"
				}
			}`,
			responseStatus: http.StatusOK,
			wantCode:       "",
			wantErr:        false,
		},
		{
			name:           "server error",
			responseBody:   "",
			responseStatus: http.StatusInternalServerError,
			wantErr:        true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				// Verify query parameters
				assert.Equal(t, "json", r.URL.Query().Get("format"))
				assert.Equal(t, "139.7671", r.URL.Query().Get("lon"))
				assert.Equal(t, "35.6812", r.URL.Query().Get("lat"))
				assert.Equal(t, "18", r.URL.Query().Get("zoom"))
				assert.Equal(t, "1", r.URL.Query().Get("addressdetails"))

				// Verify User-Agent header
				assert.NotEmpty(t, r.Header.Get("User-Agent"))

				w.WriteHeader(tt.responseStatus)
				_, _ = w.Write([]byte(tt.responseBody))
			}))
			defer server.Close()

			client := NewNominatimClient(nil, server.URL, "test-agent")
			result, err := client.Fetch(context.Background(), 139.7671, 35.6812)

			if tt.wantErr {
				assert.Error(t, err)
				return
			}

			assert.NoError(t, err)
			if tt.wantCode == "" {
				assert.True(t, result == nil || result.MunicipalityCode == "")
			} else {
				assert.NotNil(t, result)
				assert.Equal(t, tt.wantCode, result.MunicipalityCode)
			}
		})
	}
}

func TestNominatimClient_Fetch_RateLimit(t *testing.T) {
	requests := 0
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		requests++
		_, _ = w.Write([]byte(`{"address": null}`))
	}))
	defer server.Close()

	client := NewNominatimClient(nil, server.URL, "test-agent")

	_, err := client.Fetch(context.Background(), 139.7671, 35.6812)
	assert.NoError(t, err)

	// The second request within the same second must not reach Nominatim.
	_, err = client.Fetch(context.Background(), 139.7671, 35.6812)
	assert.ErrorIs(t, err, ErrNominatimRateLimited)
	assert.Equal(t, 1, requests)
}

func TestNominatimClient_Fetch_TooManyRequests(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusTooManyRequests)
	}))
	defer server.Close()

	client := NewNominatimClient(nil, server.URL, "test-agent")
	_, err := client.Fetch(context.Background(), 139.7671, 35.6812)
	assert.ErrorIs(t, err, ErrNominatimRateLimited)
}
