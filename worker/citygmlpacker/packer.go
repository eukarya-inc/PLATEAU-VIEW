package citygmlpacker

import (
	"archive/zip"
	"context"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"path"
	"path/filepath"
	"slices"
	"sort"
	"strings"
	"time"

	"github.com/reearth/reearthx/log"
)

type Packer struct {
	w          io.Writer
	p          progress
	cachedir   string
	httpClient *http.Client
}

type packerContext struct {
	zw       *zip.Writer
	modified time.Time
	urls     []string
	seen     map[string]struct{}
	roots    map[string]struct{}
	files    map[string]struct{}
}

func NewPacker(w io.Writer, c *http.Client) *Packer {
	if c == nil {
		c = &http.Client{}
	}
	return &Packer{
		w:          w,
		p:          progress{},
		cachedir:   filepath.Join(os.TempDir(), "reearth-plateauview"),
		httpClient: c,
	}
}

func (p *Packer) Progress() *progress {
	return &p.p
}

func (p *Packer) Pack(ctx context.Context, host string, urls []string) error {
	p.p.Reset(int64(len(urls) * 2))

	// sort urls by extension (.gml -> .zip)
	urls = sortURLs(urls)

	zw := zip.NewWriter(p.w)
	defer zw.Close()

	pctx := &packerContext{
		zw:       zw,
		modified: time.Now(),
		urls:     urls,
		seen:     map[string]struct{}{},
		roots:    map[string]struct{}{},
		files:    map[string]struct{}{},
	}

	for len(pctx.urls) > 0 {
		uu := pctx.urls[0]
		u, err := url.Parse(uu)
		if err != nil {
			return fmt.Errorf("invalid URL: %w", err)
		}

		if u.Scheme != "http" && u.Scheme != "https" {
			return fmt.Errorf("invalid scheme: %s", u.Scheme)
		}

		if u.Host == "" || (host != "" && u.Host != host) {
			return fmt.Errorf("invalid host: %s", u.Host)
		}

		if ext := path.Ext(u.Path); ext == ".zip" {
			if err := p.writeZip(ctx, u, pctx); err != nil {
				return fmt.Errorf("write zip: %w", err)
			}
		} else if ext == ".gml" {
			err := p.writeGML(ctx, u, pctx)
			if err != nil {
				return fmt.Errorf("write gml: %w", err)
			}
		} else {
			log.Warnfc(ctx, "invalid extension: %s", ext)
		}

		pctx.urls = pctx.urls[1:]
	}

	if err := zw.Close(); err != nil {
		return fmt.Errorf("close zip: %w", err)
	}

	return nil
}

func sortURLs(urls []string) []string {
	urls = slices.Clone(urls)
	sort.SliceStable(urls, func(i, j int) bool {
		return !strings.HasSuffix(urls[i], ".zip") && strings.HasSuffix(urls[j], ".zip")
	})
	return urls
}
