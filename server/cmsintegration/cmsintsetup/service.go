package cmsintsetup

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"net/http"

	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
	cms "github.com/reearth/reearth-cms-api/go"
)

type Services struct {
	CMS        cms.Interface
	PCMS       plateaucms.FeatureTypeStore
	PlateauCMS *PlateauCMS
	HTTP       *http.Client
}

func NewServices(conf Config) (*Services, error) {
	cms, err := cms.New(conf.CMSURL, conf.CMSToken)
	if err != nil {
		return nil, err
	}

	pcms, err := plateaucms.New(plateaucms.Config{
		CMSBaseURL:       conf.CMSURL,
		CMSMainToken:     conf.CMSToken,
		CMSSystemProject: conf.CMSSystemProject,
	})
	if err != nil {
		return nil, err
	}

	return &Services{
		CMS:        cms,
		PCMS:       pcms,
		PlateauCMS: NewPlateauCMS(cms, ""),
	}, nil
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
