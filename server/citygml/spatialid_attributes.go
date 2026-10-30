package citygml

import (
	"context"
	"io"
	"net/http"
	"net/url"
	"path"
	"sync"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/geo/spatialid"
	"github.com/klauspost/compress/gzip"
	"github.com/orisano/gosax/xmlb"
	"github.com/reearth/reearthx/log"
	"golang.org/x/sync/errgroup"
)

// spatialIDConcurrency bounds the number of upstream CityGML files fetched and
// parsed in parallel by SpatialIDAttributes. Matches the sibling
// /datacatalog/citygml handler's cap so a broad, low-zoom spatial ID doesn't
// tie up an instance for minutes issuing sequential HTTP calls.
const spatialIDConcurrency = 10

// spatialIDMaxURLs caps the number of files a single request will fetch. A
// broad spatial ID over many cities can otherwise enqueue thousands of full-
// file HTTP GETs; even parallelized, that eats up the request budget and
// upstream quota. 500 covers realistic multi-city queries but stops obvious
// blowups.
const spatialIDMaxURLs = 500

type Reader interface {
	Open(ctx context.Context) (io.Reader, func() error, error)
	Resolver() CodeResolver
}

type urlReader struct {
	URL    string
	client *http.Client

	skipCodeListFetch bool
	// etagCache is shared across parallel readers within the same request,
	// so mutate it under mu.
	etagCache   map[string]string
	etagCacheMu *sync.Mutex
}

func (r *urlReader) Open(ctx context.Context) (io.Reader, func() error, error) {
	log.Debugfc(ctx, "citygml: open url: %s", r.URL)
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, r.URL, nil)
	if err != nil {
		return nil, nil, err
	}
	u, _ := url.ParseRequestURI(r.URL)
	cacheKey := path.Base(u.Path)

	req.Header.Set("Accept-Encoding", "gzip")
	// The ETag cache and its mutex are wired together — both non-nil, or
	// neither. Guarding both defends against a partially-initialised reader
	// where a caller passes an etagCache map but forgets the mutex.
	etagCacheEnabled := r.etagCache != nil && r.etagCacheMu != nil
	if etagCacheEnabled {
		r.etagCacheMu.Lock()
		if etag, ok := r.etagCache[cacheKey]; ok {
			req.Header.Set("If-None-Match", etag)
		}
		r.etagCacheMu.Unlock()
	}
	resp, err := r.client.Do(req)
	if err != nil {
		return nil, nil, err
	}
	if resp.StatusCode == http.StatusNotModified {
		return nil, nil, resp.Body.Close()
	}
	if resp.StatusCode != http.StatusOK {
		log.Debugfc(ctx, "citygml: failed to open: %s", resp.Status)
		return nil, nil, resp.Body.Close()
	}
	if etagCacheEnabled {
		r.etagCacheMu.Lock()
		r.etagCache[cacheKey] = resp.Header.Get("ETag")
		r.etagCacheMu.Unlock()
	}
	if resp.Header.Get("Content-Encoding") == "gzip" {
		gr, err := gzip.NewReader(resp.Body)
		if err != nil {
			_ = resp.Body.Close()
			return nil, nil, err
		}
		cleanup := func() error {
			_ = gr.Close()
			return resp.Body.Close()
		}
		return gr, cleanup, nil
	}
	return resp.Body, resp.Body.Close, nil
}

func (r *urlReader) Resolver() CodeResolver {
	if r.skipCodeListFetch {
		return nil
	}
	return &fetchCodeResolver{
		client: r.client,
		url:    r.URL,
	}
}

func SpatialIDAttributes(ctx context.Context, rs []Reader, spatialIDs []string, yield func(map[string]any) error) error {
	var filter lod1SolidFilter
	for _, sid := range spatialIDs {
		v, err := spatialid.Parse(sid)
		if err != nil {
			return err
		}
		filter.Bounds = append(filter.Bounds, v.Bounds())
	}

	if len(filter.Bounds) == 0 {
		return nil
	}

	// Fetch and parse each URL concurrently. Each goroutine owns its own
	// XML-decoder buffer and tag-handler cache; the caller's yield is
	// serialized under yieldMu so JSON output stays well-formed.
	g, gctx := errgroup.WithContext(ctx)
	g.SetLimit(spatialIDConcurrency)
	var yieldMu sync.Mutex
	safeYield := func(v map[string]any) error {
		yieldMu.Lock()
		defer yieldMu.Unlock()
		return yield(v)
	}

	for _, r := range rs {
		r := r
		g.Go(func() error {
			return processSpatialIDReader(gctx, r, filter, safeYield)
		})
	}

	return g.Wait()
}

func processSpatialIDReader(ctx context.Context, r Reader, filter lod1SolidFilter, yield func(map[string]any) error) error {
	rc, cleanup, err := r.Open(ctx)
	if err != nil {
		return err
	}
	if rc == nil {
		log.Debugfc(ctx, "citygml: skip scan")
		return nil
	}
	defer func() {
		_ = cleanup()
	}()

	// Buffer and tag-handler cache are per-goroutine — sharing them across
	// concurrent parses would corrupt scanner state.
	buf := make([]byte, 32*1024)
	h := lod1SolidHandler{
		Filter: filter,
	}
	fs := &featureScanner{
		Dec: xmlb.NewDecoder(rc, buf),
	}
	count, matched := 0, 0
	thCache := map[string]tagHandler{}
	for fs.Scan() {
		if err := ctx.Err(); err != nil {
			return err
		}
		count++

		id, el := fs.Feature()
		tag := tagName(el.Name)
		if _, ok := thCache[tag]; !ok {
			thCache[tag] = toTagHandler(tag, schemaDefs, r.Resolver())
		}
		fah, err := newFeatureAttributeHandler(fs.ns, id, tag, thCache[tag])
		if err != nil {
			return err
		}
		h.Next = fah
		h.boundingBox = nil // Reset bounding box for each feature
		ok, err := processFeature(fs.Dec, &h)
		if err != nil {
			return err
		}
		if ok {
			if h.boundingBox != nil {
				fah.Val["_bbox"] = map[string]any{
					"min": map[string]float64{
						"lng": h.boundingBox.Min.X,
						"lat": h.boundingBox.Min.Y,
						"alt": h.boundingBox.Min.Z,
					},
					"max": map[string]float64{
						"lng": h.boundingBox.Max.X,
						"lat": h.boundingBox.Max.Y,
						"alt": h.boundingBox.Max.Z,
					},
					"center": map[string]float64{
						"lng": (h.boundingBox.Min.X + h.boundingBox.Max.X) / 2,
						"lat": (h.boundingBox.Min.Y + h.boundingBox.Max.Y) / 2,
						"alt": (h.boundingBox.Min.Z + h.boundingBox.Max.Z) / 2,
					},
				}
			}
			matched++
			if err := yield(fah.Val); err != nil {
				return err
			}
		}
	}

	if err := fs.Err(); err != nil {
		return err
	}

	log.Debugfc(ctx, "citygml: %d features scanned and %d intersected", count, matched)
	return nil
}
