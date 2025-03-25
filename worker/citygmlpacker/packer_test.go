package citygmlpacker

import (
	"archive/zip"
	"bytes"
	"io"
	"testing"

	"github.com/jarcoal/httpmock"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestPacker_Pack(t *testing.T) {
	const testgml1 = `<?xml version="1.0" encoding="UTF-8"?>` // no deps
	const testgml2 = `<?xml version="1.0" encoding="UTF-8"?>
<app:imageURI>image.png</app:imageURI>
<element codeSpace="../../codelists/dep.gml" />`
	const testgml3 = `<?xml version="1.0" encoding="UTF-8"?>`
	const testimg = `hoge` // it's not a real image but it doesn't matter

	zbuf := bytes.NewBuffer(nil)
	zw := zip.NewWriter(zbuf)
	f, _ := zw.Create("codelists/hoge.gml")
	_, _ = f.Write([]byte("foobar"))
	f, _ = zw.Create("00000_yyy/misc/codelists/foo.gml")
	_, _ = f.Write([]byte("barfoo"))
	_ = zw.Close()

	httpmock.Activate()
	defer httpmock.DeactivateAndReset()

	httpmock.RegisterResponder("GET", "http://example.com/assets/xx/xxxxx/00000_example_citygml/udx/bldg/1.gml",
		httpmock.NewStringResponder(200, testgml1))
	httpmock.RegisterResponder("GET", "http://example.com/assets/xx/xxxxx/00000_example_citygml/udx/bldg/2.gml",
		httpmock.NewStringResponder(200, testgml2))
	httpmock.RegisterResponder("GET", "http://example.com/assets/xx/xxxxx/00000_example_citygml/codelists/dep.gml",
		httpmock.NewStringResponder(200, testgml3))
	httpmock.RegisterResponder("GET", "http://example.com/assets/xx/xxxxx/00000_example_citygml/udx/bldg/image.png",
		httpmock.NewStringResponder(200, testimg))
	httpmock.RegisterResponder("GET", "http://example.com/assets/xx/xxxxx/00000_yyy.zip",
		httpmock.NewBytesResponder(200, zbuf.Bytes()))

	buf := bytes.NewBuffer(nil)
	urls := []string{
		"http://example.com/assets/xx/xxxxx/00000_yyy.zip",
		"http://example.com/assets/xx/xxxxx/00000_example_citygml/udx/bldg/1.gml",
		"http://example.com/assets/xx/xxxxx/00000_example_citygml/udx/bldg/2.gml",
	}
	packer := NewPacker(buf, nil)

	err := packer.Pack(t.Context(), "example.com", urls)
	require.NoError(t, err)

	// unzip
	zr, err := zip.NewReader(bytes.NewReader(buf.Bytes()), int64(buf.Len()))
	require.NoError(t, err)

	assert.Len(t, zr.File, 6)
	assert.Equal(t, "00000_example_citygml/udx/bldg/1.gml", zr.File[0].Name)
	assert.Equal(t, "00000_example_citygml/udx/bldg/2.gml", zr.File[1].Name)
	assert.Equal(t, "00000_example_citygml/codelists/dep.gml", zr.File[2].Name)
	assert.Equal(t, "00000_example_citygml/udx/bldg/image.png", zr.File[3].Name)
	assert.Equal(t, "00000_example_citygml/codelists/hoge.gml", zr.File[4].Name)
	assert.Equal(t, "00000_example_citygml/codelists/foo.gml", zr.File[5].Name)

	b := string(lo.Must(io.ReadAll(lo.Must(zr.File[0].Open()))))
	assert.Equal(t, testgml1, b, "1.gml")

	b = string(lo.Must(io.ReadAll(lo.Must(zr.File[1].Open()))))
	assert.Equal(t, testgml2, b, "2.gml")

	b = string(lo.Must(io.ReadAll(lo.Must(zr.File[2].Open()))))
	assert.Equal(t, testgml3, b, "dep.gml")

	b = string(lo.Must(io.ReadAll(lo.Must(zr.File[3].Open()))))
	assert.Equal(t, testimg, b, "image.png")

	b = string(lo.Must(io.ReadAll(lo.Must(zr.File[4].Open()))))
	assert.Equal(t, "foobar", b, "hoge.gml")

	b = string(lo.Must(io.ReadAll(lo.Must(zr.File[5].Open()))))
	assert.Equal(t, "barfoo", b, "foo.gml")
}

func Test_SortURLs(t *testing.T) {
	urls := []string{
		"http://example.com/a.gml",
		"http://example.com/b.zip",
		"http://example.com/c.gml",
		"http://example.com/d.zip",
		"http://example.com/e.gml",
	}
	urls = sortURLs(urls)
	assert.Equal(t, []string{
		"http://example.com/a.gml",
		"http://example.com/c.gml",
		"http://example.com/e.gml",
		"http://example.com/b.zip",
		"http://example.com/d.zip",
	}, urls)
}
