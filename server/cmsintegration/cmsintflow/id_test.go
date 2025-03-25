package cmsintflow

import (
	"testing"

	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestID(t *testing.T) {
	i := ID{ItemID: "item", FeatureType: "bldg", ProjectID: "project", Type: "qc_conv"}
	assert.Equal(t, i, lo.Must(parseID(i.Sign("aaa"), "aaa")))
	_, err := parseID(i.Sign("aaa"), "aaa2")
	assert.Same(t, ErrInvalidID, err)
}
