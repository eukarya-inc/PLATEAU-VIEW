package cmsintsetup

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestCutByOffsetAndLimit(t *testing.T) {
	s := []int{1, 2, 3, 4, 5}

	assert.Equal(t, []int{1, 2, 3, 4, 5}, cutByOffsetAndLimit(s, 0, 0))
	assert.Equal(t, []int{3, 4, 5}, cutByOffsetAndLimit(s, 2, 0))
	assert.Equal(t, []int{1, 2}, cutByOffsetAndLimit(s, 0, 2))
	assert.Equal(t, []int{3, 4}, cutByOffsetAndLimit(s, 2, 2))
	assert.Equal(t, []int{4, 5}, cutByOffsetAndLimit(s, 3, 10))
	assert.Empty(t, cutByOffsetAndLimit(s, 10, 2))
}
