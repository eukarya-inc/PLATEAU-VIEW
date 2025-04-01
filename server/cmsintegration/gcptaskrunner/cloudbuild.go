package gcptaskrunner

import (
	"context"
	"path"

	"google.golang.org/api/cloudbuild/v1"
)

func RunCloudBuild(ctx context.Context, task Task, conf Config) error {
	if task.Image == "" {
		task.Image = conf.Task.Image
	}

	if task.Image == "" {
		return ErrImageEmpty
	}

	cb, err := cloudbuild.NewService(ctx)
	if err != nil {
		return err
	}

	machineType := ""
	if v := conf.MachineType; v != "default" {
		machineType = v
	}

	build := &cloudbuild.Build{
		Timeout:  conf.Timeout,
		QueueTtl: conf.QueueTtl,
		Steps: []*cloudbuild.BuildStep{
			{
				Name: task.Image,
				Args: task.Args,
				Env:  envToSlices(mergeEnv(conf.Env, task.Env)),
			},
		},
		Options: &cloudbuild.BuildOptions{
			MachineType: machineType,
			DiskSizeGb:  conf.DiskSizeGb,
		},
		Tags: conf.Tags,
	}

	call := cb.Projects.Locations.Builds.Create(
		path.Join("projects", conf.Project, "locations", conf.Region),
		build,
	)
	_, err = call.Do()

	if err != nil {
		return err
	}
	return nil
}
