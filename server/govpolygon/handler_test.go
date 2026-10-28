package govpolygon

import (
	"context"
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

func TestHandlerUpdateLeaderCanceled(t *testing.T) {
	release := make(chan struct{})
	s := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		<-release
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"data":{"areas":[{"name":"千代田区","code":"13101","prefecture":{"name":"東京都"}}]}}`))
	}))
	defer s.Close()

	h := New(s.URL, true)
	e := echo.New()

	// the leader request is canceled while the upstream query is in flight
	ctx, cancel := context.WithCancel(context.Background())
	r := httptest.NewRequest(http.MethodGet, "/", nil).WithContext(ctx)
	done := make(chan error, 1)
	go func() {
		done <- h.Update(e.NewContext(r, httptest.NewRecorder()))
	}()

	time.Sleep(50 * time.Millisecond)
	cancel()
	close(release)

	assert.NoError(t, <-done)

	h.lock.RLock()
	defer h.lock.RUnlock()
	assert.True(t, h.failedAt.IsZero())
	assert.NotNil(t, h.geojson)
}

func TestHandlerGetCityNamesErrors(t *testing.T) {
	for name, body := range map[string]string{
		"graphql error": `{"errors":[{"message":"boom"}],"data":null}`,
		"no areas":      `{"data":{"areas":[]}}`,
	} {
		t.Run(name, func(t *testing.T) {
			s := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
				w.Header().Set("Content-Type", "application/json")
				_, _ = w.Write([]byte(body))
			}))
			defer s.Close()

			names, err := New(s.URL, true).getCityNames(context.Background())
			assert.Error(t, err)
			assert.Nil(t, names)
		})
	}
}

func TestHandlerRefreshAfterCacheDuration(t *testing.T) {
	var requests int32
	s := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		atomic.AddInt32(&requests, 1)
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"data":{"areas":[{"name":"千代田区","code":"13101","prefecture":{"name":"東京都"}}]}}`))
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

	assert.Equal(t, http.StatusOK, get())
	assert.Equal(t, int32(1), atomic.LoadInt32(&requests))

	// fresh data is not refreshed
	assert.Equal(t, http.StatusOK, get())
	assert.Equal(t, int32(1), atomic.LoadInt32(&requests))

	// stale data is still served and refreshed in the background
	h.lock.Lock()
	h.updatedAt = time.Now().Add(-2 * cacheDuration)
	h.lock.Unlock()

	assert.Equal(t, http.StatusOK, get())
	assert.Eventually(t, func() bool {
		h.lock.RLock()
		defer h.lock.RUnlock()
		return time.Since(h.updatedAt) < cacheDuration
	}, 5*time.Second, 10*time.Millisecond)
	assert.Equal(t, int32(2), atomic.LoadInt32(&requests))
}
