package cmsintmaxlod

import (
	"context"
	"fmt"

	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/gcptaskrunner"
	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
	cms "github.com/reearth/reearth-cms-api/go"
)

type Services struct {
	CMS        *cms.CMS
	PCMS       *plateaucms.CMS
	TaskRunner gcptaskrunner.TaskRunner
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

	if c.GCPProject != "" {
		image := c.WorkerImage

		s.TaskRunner = gcptaskrunner.NewGCPTaskRunner(gcptaskrunner.Config{
			Service: gcptaskrunner.ServiceCloudBuild,
			Project: c.GCPProject,
			Region:  c.GCPRegion,
			Task: gcptaskrunner.Task{
				Image: image,
			},
			Env: map[string]string{
				"REEARTH_CMS_URL":   c.CMSBaseURL,
				"REEARTH_CMS_TOKEN": c.CMSToken,
				"NO_COLOR":          "true",
			},
			DiskSizeGb: 1000,
			Timeout:    "86400s", // 1 day
			QueueTtl:   "86400s", // 1 day
		})
	}

	return
}

func (s *Services) GetMainItemWithMetadata(ctx context.Context, item *cms.Item) (*cms.Item, error) {
	return cmsintegrationcommon.GetMainItemWithMetadata(ctx, s.CMS, item)
}
