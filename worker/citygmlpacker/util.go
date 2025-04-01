package citygmlpacker

import (
	"compress/gzip"
	"context"
	"fmt"
	"io"
	"net/http"
	"net/url"
)

type multiReadCloser struct {
	main      io.ReadCloser
	secondary io.Closer
}

func (m *multiReadCloser) Read(p []byte) (n int, err error) {
	return m.main.Read(p)
}

func (m *multiReadCloser) Close() error {
	if err := m.main.Close(); err != nil {
		return err
	}
	return m.secondary.Close()
}

func httpGet(ctx context.Context, c *http.Client, url string) (io.ReadCloser, error) {
	req, err := http.NewRequestWithContext(ctx, "GET", url, nil)
	if err != nil {
		return nil, fmt.Errorf("invalid url: %w", err)
	}

	req.Header.Set("Accept-Encoding", "gzip")

	resp, err := c.Do(req)
	if err != nil {
		return nil, fmt.Errorf("request: %w", err)
	}

	if resp.StatusCode == http.StatusNotFound {
		_ = resp.Body.Close()
		return nil, nil
	}

	if resp.StatusCode != http.StatusOK {
		return resp.Body, fmt.Errorf("status code: %d", resp.StatusCode)
	}

	if resp.Header.Get("Content-Encoding") == "gzip" {
		gr, err := gzip.NewReader(resp.Body)
		if err != nil {
			return resp.Body, fmt.Errorf("gzip.NewReader: %w", err)
		}

		r := &multiReadCloser{
			main:      gr,
			secondary: resp.Body,
		}
		return r, nil
	}

	return resp.Body, nil
}

func newHTTPGetRequest(ctx context.Context, u *url.URL) *http.Request {
	return (&http.Request{
		Method:     "GET",
		URL:        u,
		Proto:      "HTTP/1.1",
		ProtoMajor: 1,
		ProtoMinor: 1,
		Header:     make(http.Header),
		Body:       nil,
		Host:       u.Host,
	}).WithContext(ctx)
}
