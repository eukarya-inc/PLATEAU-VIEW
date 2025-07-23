package gcptaskrunner

import (
	"context"
	"fmt"
	"strings"
	"time"

	batch "cloud.google.com/go/batch/apiv1"
	"cloud.google.com/go/batch/apiv1/batchpb"
	"google.golang.org/protobuf/types/known/durationpb"
)

func RunCloudBatch(ctx context.Context, task Task, conf Config) error {
	if task.Image == "" {
		return ErrImageEmpty
	}

	batchClient, err := batch.NewClient(ctx)
	if err != nil {
		return fmt.Errorf("NewClient: %w", err)
	}
	defer func() { _ = batchClient.Close() }()

	container := &batchpb.Runnable_Container{
		ImageUri: task.Image,
		Commands: task.Args,
	}

	jobName := task.JobName
	if jobName == "" {
		jobName = task.Image
	}

	// diskSizeGb := conf.DiskSizeGb
	// if diskSizeGb <= 0 {
	// 	diskSizeGb = 10
	// }

	job := batchpb.Job{
		TaskGroups: []*batchpb.TaskGroup{
			{
				TaskCount: 1,
				TaskSpec: &batchpb.TaskSpec{
					Runnables: []*batchpb.Runnable{{
						Executable: &batchpb.Runnable_Container_{Container: container},
					}},
					Environment: &batchpb.Environment{
						Variables: mergeEnv(conf.Env, task.Env),
					},
					MaxRunDuration: &durationpb.Duration{
						Seconds: int64(parseDuration(conf.Timeout) / time.Second),
					},
					MaxRetryCount: 2,
					// Volumes: []*batchpb.Volume{
					// 	{
					// 		Source:       &batchpb.Volume_DeviceName{DeviceName: "ssd"},
					// 		MountPath:    "/mnt/disks/ssd",
					// 		MountOptions: []string{"rw", "async"},
					// 	},
					// },
				},
			},
		},
		AllocationPolicy: &batchpb.AllocationPolicy{
			Instances: []*batchpb.AllocationPolicy_InstancePolicyOrTemplate{{
				PolicyTemplate: &batchpb.AllocationPolicy_InstancePolicyOrTemplate_Policy{
					Policy: &batchpb.AllocationPolicy_InstancePolicy{
						MachineType: conf.MachineType,
						// Disks: []*batchpb.AllocationPolicy_AttachedDisk{
						// 	{
						// 		DeviceName: "ssd",
						// 		Attached: &batchpb.AllocationPolicy_AttachedDisk_NewDisk{
						// 			NewDisk: &batchpb.AllocationPolicy_Disk{
						// 				Type:   "local-ssd",
						// 				SizeGb: diskSizeGb,
						// 			},
						// 		},
						// 	},
						// },
					},
				},
			}},
		},
		Labels: map[string]string{
			"tags": strings.Join(conf.Tags, ","),
		},
		LogsPolicy: &batchpb.LogsPolicy{
			Destination: batchpb.LogsPolicy_CLOUD_LOGGING,
		},
	}

	req := &batchpb.CreateJobRequest{
		Parent: fmt.Sprintf("projects/%s/locations/%s", conf.Project, conf.Region),
		JobId:  jobName,
		Job:    &job,
	}

	_, err = batchClient.CreateJob(ctx, req)
	if err != nil {
		return fmt.Errorf("unable to create job: %w", err)
	}

	return nil
}
