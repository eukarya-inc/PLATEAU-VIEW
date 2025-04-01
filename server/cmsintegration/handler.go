package cmsintegration

import (
	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationv2"
	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationv2/geospatialjpv2"
	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationv3"
	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintflow"
	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintmaxlod"
	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintrelated"
	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintsetup"
	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/dataconv"
	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/geospatialjpv3"
	"github.com/labstack/echo/v4"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
)

type Config = cmsintegrationcommon.Config

func Handler(conf Config, g *echo.Group) error {
	// flow
	if err := cmsintflow.Handler(flowConfig(conf), g); err != nil {
		return err
	}

	// v3
	if err := cmsintegrationv3.Handler(conf, g); err != nil {
		return err
	}

	// setup
	if err := cmsintsetup.Handler(setupConfig(conf), g); err != nil {
		return err
	}

	// v2 (compat)
	return compatHandler(conf, g)
}

func compatHandler(conf Config, g *echo.Group) error {
	v2, err := cmsintegrationv2.NotifyHandler(conf)
	if err != nil {
		return err
	}

	geo, err := geospatialjpv2.Handler(geospatialjpv2Config(conf))
	if err != nil {
		return err
	}

	dataconv, err := dataconv.Handler(dataConvConfig(conf))
	if err != nil {
		return err
	}

	g.POST("/notify_fme", v2)
	g.POST("/publish_to_geospatialjp", geo)
	g.POST("/dataconv", echo.WrapHandler(dataconv))
	return nil
}

func WebhookHandler(conf Config) (cmswebhook.Handler, error) {
	hflow, err := cmsintflow.WebhookHandler(flowConfig(conf))
	if err != nil {
		return nil, err
	}

	hv3, err := cmsintegrationv3.WebhookHandler(conf)
	if err != nil {
		return nil, err
	}

	hv3geo, err := geospatialjpv3.WebhookHandler(geospatialjpv3Config(conf))
	if err != nil {
		return nil, err
	}

	hmaxlod, err := cmsintmaxlod.WebhookHandler(maxlodConfig(conf))
	if err != nil {
		return nil, err
	}

	hrelated, err := cmsintrelated.WebhookHandler(relatedConfig(conf))
	if err != nil {
		return nil, err
	}

	// compat
	hv2, err := cmsintegrationv2.WebhookHandler(conf)
	if err != nil {
		return nil, err
	}

	// compat
	hv2geo, err := geospatialjpv2.WebhookHandler(geospatialjpv2Config(conf))
	if err != nil {
		return nil, err
	}

	// compat
	hv2dataconv, err := dataconv.WebhookHandler(dataConvConfig(conf))
	if err != nil {
		return nil, err
	}

	return cmswebhook.MergeHandlers([]cmswebhook.Handler{
		hflow, hv3, hv3geo, hmaxlod, hrelated,
		// compat
		hv2, hv2geo, hv2dataconv,
	}), nil
}

func maxlodConfig(conf Config) cmsintmaxlod.Config {
	return cmsintmaxlod.Config{
		CMSBaseURL:       conf.CMSBaseURL,
		CMSToken:         conf.CMSToken,
		CMSSystemProject: conf.CMSSystemProject,
		CMSIntegration:   conf.CMSIntegration,
		GCPProject:       conf.GCPProject,
		GCPRegion:        conf.GCPRegion,
		WorkerImage:      conf.TaskImage,
	}
}

func relatedConfig(conf Config) cmsintrelated.Config {
	return cmsintrelated.Config{
		CMSBaseURL:       conf.CMSBaseURL,
		CMSToken:         conf.CMSToken,
		CMSSystemProject: conf.CMSSystemProject,
		CMSIntegration:   conf.CMSIntegration,
	}
}

func geospatialjpv2Config(conf Config) geospatialjpv2.Config {
	return geospatialjpv2.Config{
		CMSBase:             conf.CMSBaseURL,
		CMSToken:            conf.CMSToken,
		CMSIntegration:      conf.CMSIntegration,
		CkanBase:            conf.CkanBaseURL,
		CkanOrg:             conf.CkanOrg,
		CkanToken:           conf.CkanToken,
		CkanPrivate:         conf.CkanPrivate,
		DisablePublication:  conf.DisableGeospatialjpPublication,
		DisableCatalogCheck: conf.DisableGeospatialjpCatalogCheck,
		PublicationToken:    conf.APIToken,
		// EnablePulicationOnWebhook: true,
	}
}

func geospatialjpv3Config(conf Config) geospatialjpv3.Config {
	return geospatialjpv3.Config{
		CMSBase:               conf.CMSBaseURL,
		CMSToken:              conf.CMSToken,
		CMSSystemProject:      conf.CMSSystemProject,
		CMSIntegration:        conf.CMSIntegration,
		CkanBase:              conf.CkanBaseURL,
		CkanOrg:               conf.CkanOrg,
		CkanToken:             conf.CkanToken,
		BuildType:             conf.GeospatialjpBuildType,
		CloudRunJobsJobName:   conf.GeospatialjpCloudRunJobsJobName,
		CloudBuildImage:       conf.TaskImage,
		CloudBuildMachineType: conf.GeospatialjpCloudBuildMachineType,
		CloudBuildProject:     conf.GeospatialjpCloudBuildProject,
		CloudBuildRegion:      conf.GeospatialjpCloudBuildRegion,
		CloudBuildDiskSizeGb:  conf.GeospatialjpCloudBuildDiskSizeGb,
	}
}

func dataConvConfig(conf Config) dataconv.Config {
	return dataconv.Config{
		Disable:  conf.DisableDataConv,
		CMSBase:  conf.CMSBaseURL,
		CMSToken: conf.CMSToken,
		APIToken: conf.APIToken,
		// CMSModel: conf.CMSModel,
	}
}

func flowConfig(conf Config) cmsintflow.Config {
	return cmsintflow.Config{
		Host:             conf.Host,
		CMSBaseURL:       conf.CMSBaseURL,
		CMSToken:         conf.CMSToken,
		CMSSystemProject: conf.CMSSystemProject,
		CMSIntegration:   conf.CMSIntegration,
		FlowBaseURL:      conf.FlowBaseURL,
		FlowToken:        conf.FlowToken,
		Secret:           conf.Secret,
	}
}

func setupConfig(conf Config) cmsintsetup.Config {
	return cmsintsetup.Config{
		Token:            conf.APIToken,
		CMSURL:           conf.CMSBaseURL,
		CMSToken:         conf.CMSToken,
		CMSSystemProject: conf.CMSSystemProject,
	}
}
