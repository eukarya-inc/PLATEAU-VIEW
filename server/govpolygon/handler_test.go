package govpolygon

import (
	"net/http"
	"net/http/httptest"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	"github.com/labstack/echo/v4"
	"github.com/stretchr/testify/assert"
)

func TestHandler(t *testing.T) {
	url := ""
	if url == "" {
		t.Skip("skipping test; no URL provided")
	}
	h := New(url, true)

	e := echo.New()
	r := httptest.NewRequest(http.MethodGet, "/", nil)
	w := httptest.NewRecorder()
	c := e.NewContext(r, w)

	assert.NoError(t, h.GetGeoJSON(c))

	assert.Equal(t, http.StatusOK, w.Code)
	body := w.Body.String()
	assert.NotEmpty(t, body)

	t.Log(body)
}

func TestHandlerUpdateFailure(t *testing.T) {
	var requests int32
	s := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		atomic.AddInt32(&requests, 1)
		w.WriteHeader(http.StatusInternalServerError)
	}))
	defer s.Close()

	h := New(s.URL, true)
	get := func() int {
		e := echo.New()
		r := httptest.NewRequest(http.MethodGet, "/", nil)
		w := httptest.NewRecorder()
		assert.NoError(t, h.GetGeoJSON(e.NewContext(r, w)))
		return w.Code
	}

	// concurrent requests must not stampede the upstream: they get a fast 404
	// and only a single query is issued.
	var wg sync.WaitGroup
	for range 10 {
		wg.Add(1)
		go func() {
			defer wg.Done()
			assert.Equal(t, http.StatusNotFound, get())
		}()
	}
	wg.Wait()
	assert.Equal(t, int32(1), atomic.LoadInt32(&requests))

	// the failure is negative-cached, so a later request does not retry either
	assert.Equal(t, http.StatusNotFound, get())
	assert.Equal(t, int32(1), atomic.LoadInt32(&requests))

	// once the negative cache expires, the update is retried
	h.lock.Lock()
	h.failedAt = time.Now().Add(-2 * failureCacheDuration)
	h.lock.Unlock()

	assert.Equal(t, http.StatusNotFound, get())
	assert.Equal(t, int32(2), atomic.LoadInt32(&requests))
}
