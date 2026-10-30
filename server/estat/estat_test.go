package estat

import (
	"context"
	"encoding/binary"
	"math"
	"slices"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

type fakeStore struct {
	docs       []areaDoc
	geometries map[string]map[string]any
	inCalls    [][]string
}

func (s *fakeStore) findIn(_ context.Context, fields, tokens []string, limit int) ([]areaDoc, error) {
	s.inCalls = append(s.inCalls, fields)
	var res []areaDoc
	for _, d := range s.docs {
		if !slices.ContainsFunc(fields, func(f string) bool {
			v, _ := d.Properties[f[len("properties."):]].(string)
			return !slices.Contains(tokens, v)
		}) {
			res = append(res, d)
		}
	}
	return res[:min(limit, len(res))], nil
}

func (s *fakeStore) findPrefix(_ context.Context, field, prefix string, limit int) ([]areaDoc, error) {
	if field != "fullAddress" {
		return nil, nil
	}
	var res []areaDoc
	for _, d := range s.docs {
		if a := createAreas([]areaDoc{d}, nil); len(a) > 0 && len(a[0].Address) >= len(prefix) && a[0].Address[:len(prefix)] == prefix {
			res = append(res, d)
		}
	}
	return res[:min(limit, len(res))], nil
}

func (s *fakeStore) findGeometry(_ context.Context, id string) (map[string]any, error) {
	return s.geometries[id], nil
}

func doc(id, pref, city, prefName, gstName, cssName, sName string) areaDoc {
	props := map[string]any{"PREF": pref, "CITY": city, "PREF_NAME": prefName, "GST_NAME": gstName, "S_NAME": sName}
	if cssName != "" {
		props["CSS_NAME"] = cssName
	} else {
		props["CSS_NAME"] = nil
	}
	return areaDoc{ID: id, Properties: props, BBox: []float64{139, 35, 140, 36}}
}

var testDocs = []areaDoc{
	doc("1", "13", "113", "東京都", "渋谷区", "", "宇田川町"),
	doc("2", "13", "113", "東京都", "渋谷区", "", "渋谷"),
	doc("3", "13", "101", "東京都", "千代田区", "", "丸の内"),
}

func TestFinder_FindAreas(t *testing.T) {
	ctx := context.Background()

	t.Run("compound match", func(t *testing.T) {
		f := NewFinder(&fakeStore{docs: testDocs})
		areas, err := f.FindAreas(ctx, []string{"渋谷区", "宇田川町"}, 0)
		require.NoError(t, err)
		assert.Equal(t, []Area{{
			ID:                "1",
			PrefectureCode:    "13",
			MunicipalityCode:  "13113",
			Name:              "宇田川町",
			Address:           "東京都渋谷区宇田川町",
			AddressComponents: []string{"東京都", "渋谷区", "宇田川町"},
			BBox:              []float64{139, 35, 140, 36},
		}}, areas)
	})

	t.Run("deduplicates across queries and applies limit", func(t *testing.T) {
		f := NewFinder(&fakeStore{docs: testDocs})
		areas, err := f.FindAreas(ctx, []string{"東京都"}, 2)
		require.NoError(t, err)
		assert.Equal(t, []string{"1", "2"}, ids(areas))
	})

	t.Run("prefix fallback", func(t *testing.T) {
		f := NewFinder(&fakeStore{docs: testDocs})
		areas, err := f.FindAreas(ctx, []string{"東京都千代"}, 0)
		require.NoError(t, err)
		assert.Equal(t, []string{"3"}, ids(areas))
	})

	t.Run("no tokens", func(t *testing.T) {
		s := &fakeStore{docs: testDocs}
		areas, err := NewFinder(s).FindAreas(ctx, []string{""}, 0)
		require.NoError(t, err)
		assert.Empty(t, areas)
		assert.Empty(t, s.inCalls)
	})

	t.Run("drops filters beyond the disjunction limit", func(t *testing.T) {
		s := &fakeStore{}
		tokens := []string{"a", "b", "c", "d", "e", "f"} // 6*6 = 36 > 30
		_, err := NewFinder(s).FindAreas(ctx, tokens, 0)
		require.NoError(t, err)
		assert.Equal(t, []string{"properties.S_NAME"}, s.inCalls[0])
	})
}

func TestFinder_FindGeometry(t *testing.T) {
	ring := packRing([2]float64{139.5, 35.5}, [2]float64{139.6, 35.5}, [2]float64{139.5, 35.5})
	s := &fakeStore{geometries: map[string]map[string]any{
		"p": {"type": "Polygon", "coordinates": map[string]any{"0": ring, "length": int64(1)}},
		"m": {"type": "MultiPolygon", "coordinates": map[string]any{
			"0":      map[string]any{"0": ring, "length": int64(1)},
			"length": float64(1),
		}},
	}}
	f := NewFinder(s)
	want := [][2]float64{{139.5, 35.5}, {139.6, 35.5}, {139.5, 35.5}}

	g, err := f.FindGeometry(context.Background(), "p")
	require.NoError(t, err)
	assert.Equal(t, &AreaGeometry{ID: "p", Geometry: Geometry{Type: "Polygon", Coordinates: [][][2]float64{want}}}, g)

	g, err = f.FindGeometry(context.Background(), "m")
	require.NoError(t, err)
	assert.Equal(t, &AreaGeometry{ID: "m", Geometry: Geometry{Type: "MultiPolygon", Coordinates: [][][][2]float64{{want}}}}, g)

	g, err = f.FindGeometry(context.Background(), "none")
	require.NoError(t, err)
	assert.Nil(t, g)

	_, err = unpackGeometry(map[string]any{"type": "Polygon", "coordinates": map[string]any{"0": []byte{1}, "length": int64(1)}})
	assert.Error(t, err)
}

func packRing(points ...[2]float64) []byte {
	b := make([]byte, 0, len(points)*16)
	for _, p := range points {
		b = binary.LittleEndian.AppendUint64(b, math.Float64bits(p[0]))
		b = binary.LittleEndian.AppendUint64(b, math.Float64bits(p[1]))
	}
	return b
}

func ids(areas []Area) []string {
	res := make([]string, len(areas))
	for i, a := range areas {
		res[i] = a.ID
	}
	return res
}
