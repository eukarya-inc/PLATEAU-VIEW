package cmsintsetup

import (
	"context"
	"fmt"
	"net/http"
	"time"

	run "cloud.google.com/go/run/apiv2"
	"cloud.google.com/go/run/apiv2/runpb"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/log"
	"google.golang.org/protobuf/types/known/durationpb"
)

const (
	workspacesModel      = "workspaces"
	citySetupCSVField    = "city_setup_csv"
	citySetupStatusField = "city_setup_status"
	// citySetupJobTimeout covers a full year of cities, which takes a few thousand CMS requests.
	citySetupJobTimeout = 3 * time.Hour
)

type CitySetupStatus string

const (
	CitySetupStatusRunning CitySetupStatus = "実行中"
	CitySetupStatusDone    CitySetupStatus = "完了"
	CitySetupStatusError   CitySetupStatus = "エラー"
)

// WorkspaceCitySetup is the part of a workspaces item in the system project that drives the city setup.
type WorkspaceCitySetup struct {
	ID        string           `cms:"id"`
	ProjectID string           `cms:"project_id,text"`
	CityCSV   *cms.PublicAsset `cms:"city_csv,asset"`
	// SetupCSV is the ID of the city CSV asset the setup was last started for.
	SetupCSV string `cms:"city_setup_csv,text"`
}

// WebhookHandler starts the city setup when a city CSV is attached to a workspaces item in the system project.
func WebhookHandler(conf Config) (cmswebhook.Handler, error) {
	if conf.CitySetupJobName == "" {
		return func(*http.Request, *cmswebhook.Payload) error { return nil }, nil
	}

	s, err := NewServices(conf)
	if err != nil {
		return nil, err
	}

	return func(req *http.Request, w *cmswebhook.Payload) error {
		ctx := log.WithPrefixMessage(req.Context(), "cmsintsetup: city setup webhook: ")
		return handleCitySetupWebhook(ctx, s, w)
	}, nil
}

func handleCitySetupWebhook(ctx context.Context, s *Services, w *cmswebhook.Payload) error {
	if w == nil || w.Type != cmswebhook.EventItemCreate && w.Type != cmswebhook.EventItemUpdate {
		return nil
	}
	if w.ItemData == nil || w.ItemData.Item == nil || w.ItemData.Model == nil || w.ItemData.Model.Key != workspacesModel {
		return nil
	}

	// other projects may have a model with the same key
	model, err := s.CMS.GetModelByKey(ctx, s.SystemProject, workspacesModel)
	if err != nil {
		return fmt.Errorf("failed to get workspaces model: %w", err)
	}
	if model.ID != w.ItemData.Model.ID {
		return nil
	}

	raw, err := s.CMS.GetItem(ctx, w.ItemData.Item.ID, true)
	if err != nil {
		return fmt.Errorf("failed to get workspaces item: %w", err)
	}

	item := WorkspaceCitySetup{}
	raw.Unmarshal(&item)

	// The status updates below come back as webhooks too. They stop here because
	// the CSV has already been recorded. Clearing city_setup_csv runs the setup again.
	if item.CityCSV == nil || item.CityCSV.ID == "" || item.CityCSV.ID == item.SetupCSV {
		return nil
	}

	if item.ProjectID == "" {
		return updateCitySetup(ctx, s.CMS, item.ID, item.CityCSV.ID, CitySetupStatusError, "project_id が設定されていません。")
	}
	if item.CityCSV.URL == "" {
		return updateCitySetup(ctx, s.CMS, item.ID, item.CityCSV.ID, CitySetupStatusError, "city_csv のURLを取得できませんでした。")
	}

	if err := updateCitySetup(ctx, s.CMS, item.ID, item.CityCSV.ID, CitySetupStatusRunning, "都市アイテムの作成を開始しました。"); err != nil {
		return err
	}

	args := []string{
		"setup-city-items",
		"-project", item.ProjectID,
		"-system-project", s.SystemProject,
		"-file", item.CityCSV.URL,
		"-report-item", item.ID,
	}

	log.Infofc(ctx, "run job: %v", args)
	if err := s.RunJob(ctx, args); err != nil {
		_ = updateCitySetup(ctx, s.CMS, item.ID, item.CityCSV.ID, CitySetupStatusError, fmt.Sprintf("ジョブを起動できませんでした: %v", err))
		return fmt.Errorf("failed to run job: %w", err)
	}

	return nil
}

// ReportCitySetup writes the status of setup-city-items to the workspaces item that started it, and comments the result.
func ReportCitySetup(ctx context.Context, c cms.Interface, itemID string, res *SetupCityItemsResult, setupErr error) error {
	status, msg := CitySetupStatusDone, "都市アイテムの作成が完了しました。"
	if setupErr != nil {
		status, msg = CitySetupStatusError, "都市アイテムの作成に失敗しました。"
	}
	if res != nil {
		msg += "\n" + res.String()
	}
	if setupErr != nil {
		msg += "\n" + setupErr.Error()
	}

	return updateCitySetup(ctx, c, itemID, "", status, msg)
}

// updateCitySetup updates the status of a workspaces item, and the setup CSV when csvID is not empty.
// msg is posted as a comment on the item, so the item keeps the history of every run.
func updateCitySetup(ctx context.Context, c cms.Interface, itemID, csvID string, status CitySetupStatus, msg string) error {
	fields := []*cms.Field{
		{Key: citySetupStatusField, Type: "select", Value: string(status)},
	}
	if csvID != "" {
		fields = append(fields, &cms.Field{Key: citySetupCSVField, Type: "text", Value: csvID})
	}

	if _, err := c.UpdateItem(ctx, itemID, fields, nil); err != nil {
		return fmt.Errorf("failed to update workspaces item: %w", err)
	}

	if msg != "" {
		if err := c.CommentToItem(ctx, itemID, msg); err != nil {
			return fmt.Errorf("failed to comment to workspaces item: %w", err)
		}
	}
	return nil
}

func cloudRunJobRunner(jobName string) func(context.Context, []string) error {
	return func(ctx context.Context, args []string) error {
		client, err := run.NewJobsClient(ctx)
		if err != nil {
			return fmt.Errorf("failed to create run client: %w", err)
		}
		defer func() { _ = client.Close() }()

		_, err = client.RunJob(ctx, &runpb.RunJobRequest{
			Name: jobName,
			Overrides: &runpb.RunJobRequest_Overrides{
				ContainerOverrides: []*runpb.RunJobRequest_Overrides_ContainerOverride{
					{Args: args},
				},
				Timeout: durationpb.New(citySetupJobTimeout),
			},
		})
		return err
	}
}
