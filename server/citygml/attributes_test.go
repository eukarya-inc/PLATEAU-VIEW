package citygml

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/jarcoal/httpmock"
	"github.com/labstack/echo/v4"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

const testdata = "udx/bldg/52382287_bldg_6697_psc_op.gml"

var ids = []string{"bldg_53e2a9a9-d512-408f-8250-eae30b7523d6"}
var expected = []map[string]any{
	{
		"bldg:measuredHeight":     14.3,
		"bldg:measuredHeight_uom": "m",
		"_type": "bldg:Building",
		"gen:genericAttribute": []any{
			map[string]any{
				"name":  "風致地区",
				"type":  "string",
				"value": "第1種風致地区（大崩）",
			},
		},
		"gml:id": "bldg_53e2a9a9-d512-408f-8250-eae30b7523d6",
		"uro:buildingDataQualityAttribute": []any{map[string]any{
			"uro:srcScale_code":       []any{"1"},
			"uro:lod1HeightType_code": "2",
		}},
		"uro:buildingDetailAttribute": []any{map[string]any{
			"uro:urbanPlanType_code": "21",
			"uro:surveyYear":         "2021",
		}},
		"uro:buildingDisasterRiskAttribute": []any{map[string]any{
			"uro:description_code": "1",
			"uro:areaType_code":    "2",
		}},
		"uro:buildingIDAttribute": []any{map[string]any{
			"uro:buildingID":      "22102-bldg-354359",
			"uro:prefecture_code": "22",
			"uro:city_code":       "22102",
		}},
	},
}

func TestAttributes(t *testing.T) {
	citygml, err := os.Open("testdata/" + testdata)
	require.NoError(t, err)
	defer citygml.Close()

	attrs, err := Attributes(citygml, ids, nil)
	require.NoError(t, err)
	got := []map[string]any{}
	b, err := json.Marshal(attrs)
	require.NoError(t, err)
	require.NoError(t, json.Unmarshal(b, &got))

	assert.NoError(t, err)
	assert.Equal(t, 1, len(attrs))
	assert.Equal(t, ids[0], attrs[0]["gml:id"])
	assert.Equal(t, expected, got)
}

func TestAttributesHandlerCodeOnly(t *testing.T) {
	citygmlURL := "http://example.com/udx/bldg/52382287_bldg_6697_psc_op.gml"
	citygml, err := os.ReadFile("testdata/" + testdata)
	require.NoError(t, err)

	httpmock.RegisterResponder(http.MethodGet, citygmlURL, httpmock.NewBytesResponder(http.StatusOK, citygml))
	httpmock.Activate()
	defer httpmock.Deactivate()

	u := &url.URL{Path: "/attributes"}
	q := url.Values{}
	q.Set("url", citygmlURL)
	q.Set("id", strings.Join(ids, ","))
	q.Set("skip_code_list_fetch", "1")
	u.RawQuery = q.Encode()

	req := httptest.NewRequest(http.MethodGet, u.String(), nil)
	rec := httptest.NewRecorder()
	e := echo.New()
	c := e.NewContext(req, rec)
	assert.NoError(t, attributeHandler("")(c))

	assert.Equal(t, http.StatusOK, rec.Code)
	assert.Equal(t, "application/json", rec.Header().Get(echo.HeaderContentType))

	var j []map[string]any
	assert.NoError(t, json.Unmarshal(rec.Body.Bytes(), &j))
	assert.Equal(t, expected, j)
}

func TestAttributesHandler(t *testing.T) {
	citygmlURL := "http://example.com/udx/bldg/52382287_bldg_6697_psc_op.gml"
	citygml, err := os.ReadFile("testdata/" + testdata)

	require.NoError(t, err)

	{
		codelistsDir := "testdata/codelists"
		es, err := os.ReadDir(codelistsDir)
		require.NoError(t, err)
		for _, f := range es {
			if f.IsDir() {
				continue
			}
			if !strings.HasSuffix(f.Name(), ".xml") {
				continue
			}
			b, err := os.ReadFile(filepath.Join(codelistsDir, f.Name()))
			require.NoError(t, err)
			httpmock.RegisterResponder(http.MethodGet, "http://example.com/codelists/"+f.Name(), httpmock.NewBytesResponder(http.StatusOK, b))
		}
	}

	httpmock.RegisterResponder(http.MethodGet, citygmlURL, httpmock.NewBytesResponder(http.StatusOK, citygml))
	httpmock.Activate()
	defer httpmock.Deactivate()

	u := &url.URL{Path: "/attributes"}
	q := url.Values{}
	q.Set("url", citygmlURL)
	q.Set("id", strings.Join(ids, ","))
	u.RawQuery = q.Encode()

	req := httptest.NewRequest(http.MethodGet, u.String(), nil)
	rec := httptest.NewRecorder()
	e := echo.New()
	c := e.NewContext(req, rec)
	assert.NoError(t, attributeHandler("")(c))

	assert.Equal(t, http.StatusOK, rec.Code)
	assert.Equal(t, "application/json", rec.Header().Get(echo.HeaderContentType))

	var j []map[string]any
	assert.NoError(t, json.Unmarshal(rec.Body.Bytes(), &j))

	expected := []map[string]any{
		{
			"bldg:measuredHeight":     14.3,
			"bldg:measuredHeight_uom": "m",
			"_type": "bldg:Building",
			"gen:genericAttribute": []any{
				map[string]any{
					"name":  "風致地区",
					"type":  "string",
					"value": "第1種風致地区（大崩）",
				},
			},
			"gml:id": "bldg_53e2a9a9-d512-408f-8250-eae30b7523d6",
			"uro:buildingDataQualityAttribute": []any{map[string]any{
				"uro:srcScale":            []any{"地図情報レベル2500"},
				"uro:srcScale_code":       []any{"1"},
				"uro:lod1HeightType":      "点群から取得_中央値",
				"uro:lod1HeightType_code": "2",
			}},
			"uro:buildingDetailAttribute": []any{map[string]any{
				"uro:urbanPlanType":      "都市計画区域",
				"uro:urbanPlanType_code": "21",
				"uro:surveyYear":         "2021",
			}},
			"uro:buildingDisasterRiskAttribute": []any{map[string]any{
				"uro:description_code": "1",
				"uro:description":      "急傾斜地の崩落",
				"uro:areaType_code":    "2",
				"uro:areaType":         "土砂災害特別警戒区域（指定済）",
			}},
			"uro:buildingIDAttribute": []any{map[string]any{
				"uro:buildingID":      "22102-bldg-354359",
				"uro:prefecture_code": "22",
				"uro:prefecture":      "静岡県",
				"uro:city_code":       "22102",
				"uro:city":            "静岡県静岡市駿河区",
			}},
		},
	}
	assert.Equal(t, expected, j)
}
