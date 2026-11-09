package extractmaxlod

import (
	"fmt"

	cms "github.com/reearth/reearth-cms-api/go"
)

type Config struct {
	CMSURL       string
	CMSToken     string
	ProjectID    string
	CityItemID   []string
	CityNames    []string
	FeatureTypes []string
	Overwrite    bool
	WetRun       bool
	Clean        bool
}

// redacted returns a copy of the config that is safe to log.
func (conf Config) redacted() Config {
	if conf.CMSToken != "" {
		conf.CMSToken = "***"
	}
	return conf
}

func (conf Config) CMS() (*cms.CMS, error) {
	cms, err := cms.New(conf.CMSURL, conf.CMSToken)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize CMS client: %w", err)
	}
	return cms, nil
}
