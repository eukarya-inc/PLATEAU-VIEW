package cmsintsetup

import (
	"context"
	"errors"
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestHandleCitySetupWebhook(t *testing.T) {
	csv := map[string]any{"id": "csv", "url": "https://example.com/cities.csv"}

	tests := []struct {
		name     string
		modelKey string
		modelID  string
		fields   []*cms.Field
		jobErr   error
		wantArgs []string
		// status and log written to the workspaces item, in order
		wantStatus []CitySetupStatus
		wantCSV    string
		wantErr    bool
	}{
		{
			name:     "start the job",
			modelKey: workspacesModel,
			modelID:  "workspaces",
			fields: []*cms.Field{
				{Key: "project_id", Type: "text", Value: "prj"},
				{Key: "city_csv", Type: "asset", Value: csv},
			},
			wantArgs: []string{
				"setup-city-items",
				"-project", "prj",
				"-system-project", "system",
				"-file", "https://example.com/cities.csv",
				"-report-item", "item",
			},
			wantStatus: []CitySetupStatus{CitySetupStatusRunning},
			wantCSV:    "csv",
		},
		{
			name:     "CSV already processed",
			modelKey: workspacesModel,
			modelID:  "workspaces",
			fields: []*cms.Field{
				{Key: "project_id", Type: "text", Value: "prj"},
				{Key: "city_csv", Type: "asset", Value: csv},
				{Key: "city_setup_csv", Type: "text", Value: "csv"},
			},
		},
		{
			name:     "no CSV",
			modelKey: workspacesModel,
			modelID:  "workspaces",
			fields: []*cms.Field{
				{Key: "project_id", Type: "text", Value: "prj"},
			},
		},
		{
			name:     "no project ID",
			modelKey: workspacesModel,
			modelID:  "workspaces",
			fields: []*cms.Field{
				{Key: "city_csv", Type: "asset", Value: csv},
			},
			wantStatus: []CitySetupStatus{CitySetupStatusError},
			wantCSV:    "csv",
		},
		{
			name:     "job fails to start",
			modelKey: workspacesModel,
			modelID:  "workspaces",
			fields: []*cms.Field{
				{Key: "project_id", Type: "text", Value: "prj"},
				{Key: "city_csv", Type: "asset", Value: csv},
			},
			jobErr: errors.New("denied"),
			wantArgs: []string{
				"setup-city-items",
				"-project", "prj",
				"-system-project", "system",
				"-file", "https://example.com/cities.csv",
				"-report-item", "item",
			},
			wantStatus: []CitySetupStatus{CitySetupStatusRunning, CitySetupStatusError},
			wantCSV:    "csv",
			wantErr:    true,
		},
		{
			name:     "workspaces model of another project",
			modelKey: workspacesModel,
			modelID:  "other",
		},
		{
			name:     "other model",
			modelKey: "plateau-city",
			modelID:  "city",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var gotArgs []string
			var gotStatus []CitySetupStatus
			var gotCSV string

			s := &Services{
				SystemProject: "system",
				CMS: &cmsintegrationcommon.CMSMock{
					MockGetModelByKey: func(ctx context.Context, projectID, key string) (*cms.Model, error) {
						assert.Equal(t, "system", projectID)
						assert.Equal(t, workspacesModel, key)
						return &cms.Model{ID: "workspaces", Key: workspacesModel}, nil
					},
					MockGetItem: func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
						assert.True(t, asset)
						return &cms.Item{ID: id, Fields: tt.fields}, nil
					},
					MockUpdateItem: func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
						assert.Equal(t, "item", id)
						item := &cms.Item{Fields: fields}
						gotStatus = append(gotStatus, CitySetupStatus(*item.FieldByKey(citySetupStatusField).GetValue().String()))
						if f := item.FieldByKey(citySetupCSVField); f != nil {
							gotCSV = *f.GetValue().String()
						}
						return item, nil
					},
				},
				RunJob: func(ctx context.Context, args []string) error {
					gotArgs = args
					return tt.jobErr
				},
			}

			err := handleCitySetupWebhook(context.Background(), s, &cmswebhook.Payload{
				Type: cmswebhook.EventItemUpdate,
				ItemData: &cmswebhook.ItemData{
					Item:  &cms.Item{ID: "item"},
					Model: &cms.Model{ID: tt.modelID, Key: tt.modelKey},
				},
			})

			if tt.wantErr {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
			}
			assert.Equal(t, tt.wantArgs, gotArgs)
			assert.Equal(t, tt.wantStatus, gotStatus)
			assert.Equal(t, tt.wantCSV, gotCSV)
		})
	}
}

func TestReportCitySetup(t *testing.T) {
	var got *cms.Item
	c := &cmsintegrationcommon.CMSMock{
		MockUpdateItem: func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
			got = &cms.Item{ID: id, Fields: fields}
			return got, nil
		},
	}

	res := &SetupCityItemsResult{Created: []string{"千代田区"}}

	require.NoError(t, ReportCitySetup(context.Background(), c, "item", res, nil))
	assert.Equal(t, "item", got.ID)
	assert.Equal(t, string(CitySetupStatusDone), *got.FieldByKey(citySetupStatusField).GetValue().String())
	assert.Equal(t, res.String(), *got.FieldByKey(citySetupLogField).GetValue().String())
	// the CSV is kept, so the update does not start the setup again
	assert.Nil(t, got.FieldByKey(citySetupCSVField))

	require.NoError(t, ReportCitySetup(context.Background(), c, "item", res, errors.New("boom")))
	assert.Equal(t, string(CitySetupStatusError), *got.FieldByKey(citySetupStatusField).GetValue().String())
	assert.Equal(t, res.String()+"\nboom", *got.FieldByKey(citySetupLogField).GetValue().String())
}
