package citygml

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"testing"

	"github.com/jarcoal/httpmock"
	"github.com/labstack/echo/v4"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestFeatures(t *testing.T) {
	b, err := os.ReadFile("testdata/" + testdata)
	require.NoError(t, err)

	t.Run("full", func(t *testing.T) {
		fs, err := Features(bytes.NewReader(b), []string{"/0/0/0/0"})
		require.NoError(t, err)
		expected := []string{
			"bldg_53e2a9a9-d512-408f-8250-eae30b7523d6",
			"bldg_c77c3e2b-ffdc-4b1a-91bf-185a1b46a4d1",
			"bldg_2eb12f7a-c5d9-4145-9609-a6a0f5824368",
			"bldg_4ee4bc68-d60e-494f-8d4f-57c68f0cb312",
		}
		require.Equal(t, expected, fs)
	})
	t.Run("partial1", func(t *testing.T) {
		fs, err := Features(bytes.NewReader(b), []string{"/18/0/231815/103921"})
		require.NoError(t, err)
		expected := []string{
			"bldg_53e2a9a9-d512-408f-8250-eae30b7523d6",
			"bldg_2eb12f7a-c5d9-4145-9609-a6a0f5824368",
		}
		require.Equal(t, expected, fs)
	})
	t.Run("partial2", func(t *testing.T) {
		fs, err := Features(bytes.NewReader(b), []string{"/18/0/231813/103922"})
		require.NoError(t, err)
		expected := []string{
			"bldg_c77c3e2b-ffdc-4b1a-91bf-185a1b46a4d1",
			"bldg_4ee4bc68-d60e-494f-8d4f-57c68f0cb312",
		}
		require.Equal(t, expected, fs)
	})
	t.Run("no-aabb", func(t *testing.T) {
		fs, err := Features(bytes.NewReader(b), []string{"/23/10/7418083/3325472"})
		require.NoError(t, err)
		require.Empty(t, fs)
	})
	t.Run("line-intersection", func(t *testing.T) {
		fs, err := Features(bytes.NewReader(b), []string{"/23/10/7418083/3325473"})
		expected := []string{
			"bldg_2eb12f7a-c5d9-4145-9609-a6a0f5824368",
		}
		require.NoError(t, err)
		require.Equal(t, expected, fs)
	})
	t.Run("box-in-polygon", func(t *testing.T) {
		fs, err := Features(bytes.NewReader(b), []string{"/24/20/14836162/6650947"})
		expected := []string{
			"bldg_2eb12f7a-c5d9-4145-9609-a6a0f5824368",
		}
		require.NoError(t, err)
		require.Equal(t, expected, fs)
	})
	t.Run("empty", func(t *testing.T) {
		fs, err := Features(bytes.NewReader(b), []string{"/1/0/0/0"})
		require.NoError(t, err)
		require.Empty(t, fs)
	})
}

func TestFeaturesHandler(t *testing.T) {
	citygmlURL := "http://example.com/udx/bldg/52382287_bldg_6697_psc_op.gml"
	citygml, err := os.ReadFile("testdata/" + testdata)
	require.NoError(t, err)
	httpmock.RegisterResponder(http.MethodGet, citygmlURL, httpmock.NewBytesResponder(http.StatusOK, citygml))
	httpmock.Activate()
	defer httpmock.Deactivate()

	u := &url.URL{Path: "/features"}
	q := url.Values{}
	q.Set("url", citygmlURL)
	q.Set("sid", "/18/0/231815/103921,/18/0/231813/103922")
	u.RawQuery = q.Encode()

	req := httptest.NewRequest(http.MethodGet, u.String(), nil)
	rec := httptest.NewRecorder()
	e := echo.New()
	c := e.NewContext(req, rec)
	assert.NoError(t, featureHandler("example.com")(c))

	assert.Equal(t, http.StatusOK, rec.Code)
	assert.Equal(t, "application/json", rec.Header().Get(echo.HeaderContentType))

	var j map[string]any
	assert.NoError(t, json.Unmarshal(rec.Body.Bytes(), &j))

	expected := map[string]any{
		"featureIds": []any{
			"bldg_53e2a9a9-d512-408f-8250-eae30b7523d6",
			"bldg_c77c3e2b-ffdc-4b1a-91bf-185a1b46a4d1",
			"bldg_2eb12f7a-c5d9-4145-9609-a6a0f5824368",
			"bldg_4ee4bc68-d60e-494f-8d4f-57c68f0cb312",
		},
	}
	assert.Equal(t, expected, j)
}
