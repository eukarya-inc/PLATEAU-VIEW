package geospatialjpv3

import (
	"context"
	"errors"
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/ckan"
	"github.com/stretchr/testify/assert"
)

type failingShowPackageCkan struct {
	ckan.Interface
	err error
}

func (c failingShowPackageCkan) ShowPackage(ctx context.Context, id string) (ckan.Package, error) {
	return ckan.Package{}, c.err
}

func TestHandler_findPackage(t *testing.T) {
	ctx := context.Background()

	// not found
	h := &handler{ckan: ckan.NewMock("org", nil, nil)}
	pkg, name, err := h.findPackage(ctx, PackageName{CityCode: "13101", CityNameEn: "chiyoda-ku", Year: 2023})
	assert.NoError(t, err)
	assert.Nil(t, pkg)
	assert.NotEmpty(t, name)

	// other errors must not be taken for not found
	h = &handler{ckan: failingShowPackageCkan{err: errors.New("status code 500")}}
	pkg, _, err = h.findPackage(ctx, PackageName{CityCode: "13101", CityNameEn: "chiyoda-ku", Year: 2023})
	assert.Error(t, err)
	assert.Nil(t, pkg)
}
