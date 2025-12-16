package geocoding

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestGSIClient_Fetch(t *testing.T) {
	tests := []struct {
		name           string
		responseBody   string
		responseStatus int
		wantCode       string
		wantName       string
		wantErr        bool
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
