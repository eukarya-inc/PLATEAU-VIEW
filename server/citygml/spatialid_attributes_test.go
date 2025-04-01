package citygml

import (
	"bytes"
	"cmp"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path"
	"slices"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestSpatialIDAttributes(t *testing.T) {
	b, err := os.ReadFile("testdata/" + testdata)
	require.NoError(t, err)

	r := &testReader{
		r: bytes.NewReader(b),
	}
	ctx := context.Background()
	attributes, err := SpatialIDAttributes(ctx, []Reader{r}, []string{"/18/0/231815/103921"})
	require.NoError(t, err)
	expected := []map[string]any{
		{
			"bldg:measuredHeight":     15.4,
			"bldg:measuredHeight_uom": "m",
			"feature_type":            "bldg:Building",
			"gen:genericAttribute": []any{
				map[string]any{
					"name":  "風致地区",
					"type":  "string",
					"value": "第1種風致地区（大崩）",
				},
			},
			"gml:id": "bldg_2eb12f7a-c5d9-4145-9609-a6a0f5824368",
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
				"uro:areaType_code":    "1",
				"uro:areaType":         "土砂災害警戒区域（指定済）",
			}},
			"uro:buildingIDAttribute": []any{map[string]any{
				"uro:buildingID":      "22102-bldg-354360",
				"uro:prefecture_code": "22",
				"uro:prefecture":      "静岡県",
				"uro:city_code":       "22102",
				"uro:city":            "静岡県静岡市駿河区",
			}},
		},
		{
			"bldg:measuredHeight":     14.3,
			"bldg:measuredHeight_uom": "m",
			"feature_type":            "bldg:Building",
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
	slices.SortFunc(attributes, func(a, b map[string]any) int {
		return cmp.Compare(a["gml:id"].(string), b["gml:id"].(string))
	})

	{
		b, err = json.Marshal(attributes)
		require.NoError(t, err)
		got := []map[string]any{}
		require.NoError(t, json.Unmarshal(b, &got))
		assert.Equal(t, expected, got)
	}
}

type testReader struct {
	r io.Reader
}

func (r *testReader) Open(_ context.Context) (io.Reader, func() error, error) {
	noop := func() error { return nil }
	return r.r, noop, nil
}

func (r *testReader) Resolver() CodeResolver {
	return &testCodeResolver{}
}

type testCodeResolver struct {
	cache map[string]map[string]string
}

func (r *testCodeResolver) Resolve(codeSpace, code string) (string, error) {
	cs := path.Base(codeSpace)
	if _, ok := r.cache[cs]; !ok {
		f, err := os.Open("testdata/codelists/" + cs)
		if err != nil {
			return "", err
		}
		defer f.Close()

		cm, err := parseCodeMap(f)
		if err != nil {
			return "", err
		}
		if r.cache == nil {
			r.cache = map[string]map[string]string{}
		}
		r.cache[cs] = cm
	}
	cm := r.cache[cs]
	if v, ok := cm[code]; ok {
		return v, nil
	} else {
		return "", fmt.Errorf("not found")
	}
}
