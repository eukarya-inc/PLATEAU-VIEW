package jisx0410

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestFind(t *testing.T) {
	tests := []struct {
		name     string
		lat      float64
		lon      float64
		level    int
		expected string
	}{
		{
			name:     "Level 1 Tokyo",
			lat:      35.6895,
			lon:      139.6917,
			level:    1,
			expected: "5339",
		},
		{
			name:     "Level 2 Tokyo",
			lat:      35.6895,
			lon:      139.6917,
			level:    2,
			expected: "533945",
		},
		{
			name:     "Level 3 Tokyo",
			lat:      35.6895,
			lon:      139.6917,
			level:    3,
			expected: "53394513",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			code := Find(tt.lon, tt.lat, tt.level)
			assert.Equal(t, tt.expected, code.String(), "mesh code string should match")
		})
	}
}

func TestFindAll(t *testing.T) {
	bbox := struct {
		minLon float64
		minLat float64
		maxLon float64
		maxLat float64
	}{
		minLon: 139.69,
		minLat: 35.68,
		maxLon: 139.70,
		maxLat: 35.69,
	}

	codes := FindAll(bbox.minLon, bbox.minLat, bbox.maxLon, bbox.maxLat, 3)

	assert.NotEmpty(t, codes, "mesh code list should not be empty")

	seen := make(map[string]struct{})
	for _, code := range codes {
		str := code.String()
		_, exists := seen[str]
		assert.False(t, exists, "duplicate mesh code: %s", str)
		seen[str] = struct{}{}
	}
}
