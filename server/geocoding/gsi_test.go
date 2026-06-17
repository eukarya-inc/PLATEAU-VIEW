package geocoding

import (
	"context"
	"errors"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func TestGSIClient_Fetch(t *testing.T) {
	tests := []struct {
		name            string
		responseBody    string
		responseStatus  int
		wantCode        string
		wantName        string
		wantErr         bool
		wantUnavailable bool
	}{
		{
			name: "success",
			responseBody: `{
				"results": {
					"muniCd": "13101",
					"lv01Nm": "東京都千代田区千代田１−１"
				}
			}`,
			responseStatus: http.StatusOK,
			wantCode:       "13101",
			wantName:       "東京都千代田区千代田１−１",
			wantErr:        false,
		},
		{
			name: "no results",
			responseBody: `{
				"results": null
			}`,
			responseStatus: http.StatusOK,
			wantCode:       "",
			wantName:       "",
			wantErr:        false,
		},
		{
			name:           "server error",
			responseBody:   "",
			responseStatus: http.StatusInternalServerError,
			wantErr:        true,
		},
		{
			name:            "bad gateway",
			responseBody:    "",
			responseStatus:  http.StatusBadGateway,
			wantErr:         true,
			wantUnavailable: true,
		},
		{
			name:            "service unavailable",
			responseBody:    "",
			responseStatus:  http.StatusServiceUnavailable,
			wantErr:         true,
			wantUnavailable: true,
		},
		{
			name:            "gateway timeout",
			responseBody:    "",
			responseStatus:  http.StatusGatewayTimeout,
			wantErr:         true,
			wantUnavailable: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				// Verify query parameters
				assert.Equal(t, "139.7671", r.URL.Query().Get("lon"))
				assert.Equal(t, "35.6812", r.URL.Query().Get("lat"))

				w.WriteHeader(tt.responseStatus)
				_, _ = w.Write([]byte(tt.responseBody))
			}))
			defer server.Close()

			client := NewGSIClient(nil, server.URL)
			result, err := client.Fetch(context.Background(), 139.7671, 35.6812)

			if tt.wantErr {
				assert.Error(t, err)
				if tt.wantUnavailable {
					assert.True(t, errors.Is(err, ErrGSIUnavailable), "expected ErrGSIUnavailable error")
				}
				return
			}

			assert.NoError(t, err)
			if tt.wantCode == "" {
				assert.Nil(t, result)
			} else {
				assert.NotNil(t, result)
				assert.Equal(t, tt.wantCode, result.MunicipalityCode)
				assert.Equal(t, tt.wantName, result.Name)
			}
		})
	}
}

func TestGSIClient_Fetch_Timeout(t *testing.T) {
	// Server that hangs longer than the client timeout, simulating a
	// slow-but-alive upstream.
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		time.Sleep(200 * time.Millisecond)
		w.WriteHeader(http.StatusOK)
	}))
	defer server.Close()

	client := NewGSIClient(&http.Client{Timeout: 20 * time.Millisecond}, server.URL)
	_, err := client.Fetch(context.Background(), 139.7671, 35.6812)

	assert.Error(t, err)
	assert.True(t, errors.Is(err, ErrGSIUnavailable), "timeout should surface as ErrGSIUnavailable so the handler falls back to Nominatim")
}
