package cmsintrelated

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"net/http"

	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
	cms "github.com/reearth/reearth-cms-api/go"
)

type Services struct {
	CMS  cms.Interface
	PCMS PCMS
	HTTP *http.Client
}

type PCMS interface {
	plateaucms.SpecStore
	plateaucms.FeatureTypeStore
}

func NewServices(c Config) (s *Services, _ error) {
	s = &Services{}

	cms, err := cms.New(c.CMSBaseURL, c.CMSToken)
	if err != nil {
		return nil, fmt.Errorf("failed to init cms: %w", err)
	}
	s.CMS = cms

	pcms, err := plateaucms.New(plateaucms.Config{
		CMSBaseURL:       c.CMSBaseURL,
		CMSMainToken:     c.CMSToken,
		CMSSystemProject: c.CMSSystemProject,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to init plateau cms: %w", err)
	}
	s.PCMS = pcms

	return
}

func (s *Services) GetMainItemWithMetadata(ctx context.Context, item *cms.Item) (*cms.Item, error) {
	return cmsintegrationcommon.GetMainItemWithMetadata(ctx, s.CMS, item)
}

func (s *Services) GET(ctx context.Context, url string) (io.ReadCloser, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	client := s.HTTP
	if client == nil {
		client = http.DefaultClient
	}

	res, err := client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to download asset: %w", err)
	}

	if res.StatusCode != http.StatusOK {
		_ = res.Body.Close()
		return nil, fmt.Errorf("failed to download asset: %s", res.Status)
	}

	return res.Body, nil
}

func (s *Services) GETAsBytes(ctx context.Context, url string) ([]byte, error) {
	body, err := s.GET(ctx, url)
	if err != nil {
		return nil, err
	}

	defer func() {
		_ = body.Close()
	}()

	buf := &bytes.Buffer{}
	_, err = buf.ReadFrom(body)
	if err != nil {
		return nil, fmt.Errorf("failed to read asset: %w", err)
	}

	return buf.Bytes(), nil
}
