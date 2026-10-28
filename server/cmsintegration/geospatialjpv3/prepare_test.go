package geospatialjpv3

import (
	"context"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestPrepare_CloudRunJobsWithoutJobName(t *testing.T) {
	err := Prepare(context.Background(), "item", "project", Config{BuildType: "cloudrunjobs"}, nil, nil)
	assert.Error(t, err)
}
