package geospatialjpv3

import (
	"context"
)

func Prepare(ctx context.Context, itemID, projectID string, conf Config, featureTypes []string) error {
	if conf.BuildType == "cloudrunjobs" {
		return prepareWithCloudRunJobs(ctx, itemID, projectID, conf.CloudRunJobsJobName, featureTypes)
	} else {
		return prepareOnCloudBuild(ctx, prepareOnCloudBuildConfig{
			City:                  itemID,
			Project:               projectID,
			CMSURL:                conf.CMSBase,
			CMSToken:              conf.CMSToken,
			FeatureTypes:          featureTypes,
			CloudBuildImage:       conf.CloudBuildImage,
			CloudBuildMachineType: conf.CloudBuildMachineType,
			CloudBuildProject:     conf.CloudBuildProject,
			CloudBuildRegion:      conf.CloudBuildRegion,
			CloudBuildDiskSizeGb:  conf.CloudBuildDiskSizeGb,
		})
	}
}
