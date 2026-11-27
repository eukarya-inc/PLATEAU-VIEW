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

func TestID_CRS(t *testing.T) {
	i := ID{ItemID: "item", FeatureType: "bldg", ProjectID: "project", Type: "conv", CRS: "JGD2024"}
	assert.Equal(t, i, lo.Must(parseID(i.Sign("aaa"), "aaa")))

	// an ID without a CRS keeps the format signed before the CRS was added
	old := ID{ItemID: "item", FeatureType: "bldg", ProjectID: "project", Type: "conv"}
	assert.Equal(t, signID("flow;item;project;bldg;conv", "aaa"), old.Sign("aaa"))
}
