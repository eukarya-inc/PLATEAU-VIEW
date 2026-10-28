package citygml

import (
	"net/http"
	"net/http/httptest"
	"os"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestFetchCodeResolver_MissingCodeIsCached(t *testing.T) {
	b, err := os.ReadFile("testdata/codelists/BuildingDataQualityAttribute_lod1HeightType.xml")
	assert.NoError(t, err)

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write(b)
	}))
	defer srv.Close()

	r := &fetchCodeResolver{client: srv.Client(), url: srv.URL + "/codelists/x.xml"}

	for i := 0; i < 2; i++ {
		_, err := r.Resolve("lod1HeightType.xml", "999")
		assert.Error(t, err, "attempt %d", i)
	}

	resolved, err := r.Resolve("lod1HeightType.xml", "1")
	assert.NoError(t, err)
	assert.Equal(t, "点群から取得_最高高さ", resolved)
}
