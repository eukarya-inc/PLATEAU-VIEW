package datacatalog

import (
	"context"
	"fmt"
	"io"
	"net/http"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
)

func FetchAttrList(ctx context.Context, pcms *plateaucms.CMS, h *http.Client, majorVersion int) (io.ReadCloser, error) {
	specs, err := pcms.PlateauSpecs(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get plateau specs: %w", err)
	}

	var url string
	for _, s := range specs {
		if s.MajorVersion == majorVersion {
			u := s.AttrList
			if u == "" {
				continue
			}

			url = u
		}
	}

	if url == "" {
		return nil, nil
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	resp, err := h.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch attr list: %w", err)
	}

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("failed to fetch attr list: %s", resp.Status)
	}

	return resp.Body, nil
}
