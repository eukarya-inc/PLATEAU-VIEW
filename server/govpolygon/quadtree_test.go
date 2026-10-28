package govpolygon

import (
	"testing"

	geojson "github.com/paulmach/go.geojson"
	"github.com/stretchr/testify/assert"
)

func TestQuadtree(t *testing.T) {
	q := NewQuadtree(nil, 0)

	res, ok := q.Find(139.760296, 35.686067)
	assert.True(t, ok)
	assert.Equal(t, "13101", res)

	res, ok = q.Find(19.760296, 35.686067)
	assert.False(t, ok)
	assert.Empty(t, res)
}

func TestIsPointInPolygonFeature_Hole(t *testing.T) {
	outer := [][]float64{{0, 0}, {10, 0}, {10, 10}, {0, 10}, {0, 0}}
	hole := [][]float64{{4, 4}, {6, 4}, {6, 6}, {4, 6}, {4, 4}}

	f := geojson.NewPolygonFeature([][][]float64{outer, hole})
	assert.True(t, isPointInPolygonFeature(1, 1, f))
	assert.False(t, isPointInPolygonFeature(5, 5, f))
	assert.False(t, isPointInPolygonFeature(11, 11, f))
	assert.True(t, f.Geometry.IsPolygon(), "the feature must not be modified")

	m := geojson.NewMultiPolygonFeature([][][]float64{outer, hole}, [][][]float64{{{4.5, 4.5}, {5.5, 4.5}, {5.5, 5.5}, {4.5, 5.5}, {4.5, 4.5}}})
	assert.True(t, isPointInPolygonFeature(1, 1, m))
	assert.False(t, isPointInPolygonFeature(4.2, 4.2, m))
	assert.True(t, isPointInPolygonFeature(5, 5, m))
}

func BenchmarkQuadtree(b *testing.B) {
	q := NewQuadtree(nil, 0)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = q.Find(139.760296, 35.686067)
	}
}
