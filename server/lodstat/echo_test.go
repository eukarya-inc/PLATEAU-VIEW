package lodstat

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestAutoSelectLevel(t *testing.T) {
	tests := []struct {
		name     string
		zoom     int
		expected int
	}{
		{
			name:     "z=0 should return level 2",
			zoom:     0,
			expected: 2,
		},
		{
			name:     "z=3 should return level 2",
			zoom:     3,
			expected: 2,
		},
		{
			name:     "z=7 should return level 2",
			zoom:     7,
			expected: 2,
		},
		{
			name:     "z=8 should return level 3",
			zoom:     8,
			expected: 3,
		},
		{
			name:     "z=10 should return level 3",
			zoom:     10,
			expected: 3,
		},
		{
			name:     "z=15 should return level 3",
			zoom:     15,
			expected: 3,
		},
		{
			name:     "z=20 should return level 3",
			zoom:     20,
			expected: 3,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			actual := autoSelectLevel(tt.zoom)
			assert.Equal(t, tt.expected, actual)
		})
	}
}

func TestIsValidLevel(t *testing.T) {
	tests := []struct {
		name     string
		level    int
		expected bool
	}{
		{
			name:     "level 1 is invalid",
			level:    1,
			expected: false,
		},
		{
			name:     "level 2 is valid",
			level:    2,
			expected: true,
		},
		{
			name:     "level 3 is valid",
			level:    3,
			expected: true,
		},
		{
			name:     "level 4 is invalid",
			level:    4,
			expected: false,
		},
		{
			name:     "level 5 is invalid",
			level:    5,
			expected: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			actual := isValidLevel(tt.level)
			assert.Equal(t, tt.expected, actual)
		})
	}
}
