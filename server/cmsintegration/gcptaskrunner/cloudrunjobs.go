package gcptaskrunner

import (
	"context"
	"time"

	run "cloud.google.com/go/run/apiv2"
	"cloud.google.com/go/run/apiv2/runpb"
	"google.golang.org/protobuf/types/known/durationpb"
)

func RunCloudRunJobs(ctx context.Context, task Task, conf Config) error {
	if task.JobName == "" {
		return ErrJobNameEmpty
	}

	client, err := run.NewJobsClient(ctx)
	if err != nil {
		return err
	}
	defer func() { _ = client.Close() }()

	overrides := runpb.RunJobRequest_Overrides{
		ContainerOverrides: []*runpb.RunJobRequest_Overrides_ContainerOverride{
			{
				Args: task.Args,
				Env:  envToJobs(mergeEnv(conf.Env, task.Env)),
			},
		},
		Timeout: &durationpb.Duration{
			Seconds: int64(parseDuration(conf.Timeout) / time.Second),
		},
	}

	req := &runpb.RunJobRequest{
		Name:      task.JobName,
		Overrides: &overrides,
	}

	if _, err = client.RunJob(ctx, req); err != nil {
		return err
	}

	return nil
}

func envToJobs(env map[string]string) []*runpb.EnvVar {
	var envs []*runpb.EnvVar
	for k, v := range env {
		envs = append(envs, &runpb.EnvVar{Name: k, Values: &runpb.EnvVar_Value{
			Value: v,
		}})
	}
	return envs
}
