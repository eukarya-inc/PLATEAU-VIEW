package citygmlpacker

import (
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestFindDeps(t *testing.T) {
	gml := `<?xml version="1.0" encoding="UTF-8"?>
<Root
    xmlns:app="http://mydomain.org/app"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xsi:schemaLocation="http://www.my.domain/gml ../schemas/my.xsd">
    <app:imageURI>some-image.png</app:imageURI>
    <someElement codeSpace="my-codespace" />
</Root>`

	depsMap := make(map[string]struct{})
	err := findDeps(strings.NewReader(gml), depsMap)

	require.NoError(t, err)
	assert.Equal(t, map[string]struct{}{
		"some-image.png":    {},
		"my-codespace":      {},
		"../schemas/my.xsd": {},
	}, depsMap)
}
