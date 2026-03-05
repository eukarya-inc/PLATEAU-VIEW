package cmsintflow

import (
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/stretchr/testify/assert"
)

// tagToMap converts a status string to a map format that CMS API expects
func tagToMap(status cmsintegrationcommon.ConvertionStatus) map[string]any {
	return map[string]any{"name": string(status)}
}

func TestShouldCancelFlow(t *testing.T) {
	tests := []struct {
		name     string
		payload  *cmswebhook.Payload
		expected bool
	}{
		{
			name: "item.create event should not cancel",
			payload: &cmswebhook.Payload{
				Type: cmswebhook.EventItemCreate,
				ItemData: &cmswebhook.ItemData{
					Item: &cms.Item{
						ID:             "item1",
						OriginalItemID: strPtr("main1"),
					},
				},
			},
			expected: false,
		},
		{
			name: "item.update on main item (no OriginalItemID) should not cancel",
			payload: &cmswebhook.Payload{
				Type: cmswebhook.EventItemUpdate,
				ItemData: &cmswebhook.ItemData{
					Item: &cms.Item{
						ID:             "item1",
						OriginalItemID: nil,
					},
				},
			},
			expected: false,
		},
		{
			name: "item.update on metadata item with conv_status change to error should cancel",
			payload: &cmswebhook.Payload{
				Type: cmswebhook.EventItemUpdate,
				ItemData: &cmswebhook.ItemData{
					Item: &cms.Item{
						ID:             "metadata1",
						OriginalItemID: strPtr("main1"),
						Fields: []*cms.Field{
							{ID: "field1", Key: "conv_status", Value: &cms.Tag{Name: string(cmsintegrationcommon.ConvertionStatusError)}},
						},
					},
					Changes: []cms.FieldChange{
						{
							ID:            "field1",
							PreviousValue: tagToMap(cmsintegrationcommon.ConvertionStatusRunning),
							CurrentValue:  tagToMap(cmsintegrationcommon.ConvertionStatusError),
						},
					},
				},
			},
			expected: true,
		},
		{
			name: "item.update on metadata item with qc_status change to success should cancel",
			payload: &cmswebhook.Payload{
				Type: cmswebhook.EventItemUpdate,
				ItemData: &cmswebhook.ItemData{
					Item: &cms.Item{
						ID:             "metadata1",
						OriginalItemID: strPtr("main1"),
						Fields: []*cms.Field{
							{ID: "field1", Key: "qc_status", Value: &cms.Tag{Name: string(cmsintegrationcommon.ConvertionStatusSuccess)}},
						},
					},
					Changes: []cms.FieldChange{
						{
							ID:            "field1",
							PreviousValue: tagToMap(cmsintegrationcommon.ConvertionStatusRunning),
							CurrentValue:  tagToMap(cmsintegrationcommon.ConvertionStatusSuccess),
						},
					},
				},
			},
			expected: true,
		},
		{
			name: "item.update on metadata item with conv_status remaining running should not cancel",
			payload: &cmswebhook.Payload{
				Type: cmswebhook.EventItemUpdate,
				ItemData: &cmswebhook.ItemData{
					Item: &cms.Item{
						ID:             "metadata1",
						OriginalItemID: strPtr("main1"),
						Fields: []*cms.Field{
							{ID: "field1", Key: "conv_status", Value: &cms.Tag{Name: string(cmsintegrationcommon.ConvertionStatusRunning)}},
						},
					},
					Changes: []cms.FieldChange{
						{
							ID:            "field1",
							PreviousValue: tagToMap(cmsintegrationcommon.ConvertionStatusNotStarted),
							CurrentValue:  tagToMap(cmsintegrationcommon.ConvertionStatusRunning),
						},
					},
				},
			},
			expected: false,
		},
		{
			name: "item.update on metadata item with unrelated field change should not cancel",
			payload: &cmswebhook.Payload{
				Type: cmswebhook.EventItemUpdate,
				ItemData: &cmswebhook.ItemData{
					Item: &cms.Item{
						ID:             "metadata1",
						OriginalItemID: strPtr("main1"),
						Fields: []*cms.Field{
							{ID: "field1", Key: "some_other_field", Value: "value"},
						},
					},
					Changes: []cms.FieldChange{
						{
							ID:            "field1",
							PreviousValue: "old",
							CurrentValue:  "new",
						},
					},
				},
			},
			expected: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := shouldCancelFlow(tt.payload)
			assert.Equal(t, tt.expected, result)
		})
	}
}

func TestHasStatusChangeToNonRunning(t *testing.T) {
	tests := []struct {
		name     string
		payload  *cmswebhook.Payload
		fieldKey string
		expected bool
	}{
		{
			name: "field not found",
			payload: &cmswebhook.Payload{
				ItemData: &cmswebhook.ItemData{
					Item: &cms.Item{
						Fields: []*cms.Field{},
					},
					Changes: []cms.FieldChange{},
				},
			},
			fieldKey: "conv_status",
			expected: false,
		},
		{
			name: "field found but no change",
			payload: &cmswebhook.Payload{
				ItemData: &cmswebhook.ItemData{
					Item: &cms.Item{
						Fields: []*cms.Field{
							{ID: "field1", Key: "conv_status", Value: &cms.Tag{Name: string(cmsintegrationcommon.ConvertionStatusRunning)}},
						},
					},
					Changes: []cms.FieldChange{},
				},
			},
			fieldKey: "conv_status",
			expected: false,
		},
		{
			name: "changed to running should return false",
			payload: &cmswebhook.Payload{
				ItemData: &cmswebhook.ItemData{
					Item: &cms.Item{
						Fields: []*cms.Field{
							{ID: "field1", Key: "conv_status", Value: &cms.Tag{Name: string(cmsintegrationcommon.ConvertionStatusRunning)}},
						},
					},
					Changes: []cms.FieldChange{
						{
							ID:            "field1",
							PreviousValue: tagToMap(cmsintegrationcommon.ConvertionStatusNotStarted),
							CurrentValue:  tagToMap(cmsintegrationcommon.ConvertionStatusRunning),
						},
					},
				},
			},
			fieldKey: "conv_status",
			expected: false,
		},
		{
			name: "changed to error should return true",
			payload: &cmswebhook.Payload{
				ItemData: &cmswebhook.ItemData{
					Item: &cms.Item{
						Fields: []*cms.Field{
							{ID: "field1", Key: "conv_status", Value: &cms.Tag{Name: string(cmsintegrationcommon.ConvertionStatusError)}},
						},
					},
					Changes: []cms.FieldChange{
						{
							ID:            "field1",
							PreviousValue: tagToMap(cmsintegrationcommon.ConvertionStatusRunning),
							CurrentValue:  tagToMap(cmsintegrationcommon.ConvertionStatusError),
						},
					},
				},
			},
			fieldKey: "conv_status",
			expected: true,
		},
		{
			name: "changed to success should return true",
			payload: &cmswebhook.Payload{
				ItemData: &cmswebhook.ItemData{
					Item: &cms.Item{
						Fields: []*cms.Field{
							{ID: "field1", Key: "qc_status", Value: &cms.Tag{Name: string(cmsintegrationcommon.ConvertionStatusSuccess)}},
						},
					},
					Changes: []cms.FieldChange{
						{
							ID:            "field1",
							PreviousValue: tagToMap(cmsintegrationcommon.ConvertionStatusRunning),
							CurrentValue:  tagToMap(cmsintegrationcommon.ConvertionStatusSuccess),
						},
					},
				},
			},
			fieldKey: "qc_status",
			expected: true,
		},
		{
			name: "changed to not started should return true",
			payload: &cmswebhook.Payload{
				ItemData: &cmswebhook.ItemData{
					Item: &cms.Item{
						Fields: []*cms.Field{
							{ID: "field1", Key: "conv_status", Value: &cms.Tag{Name: string(cmsintegrationcommon.ConvertionStatusNotStarted)}},
						},
					},
					Changes: []cms.FieldChange{
						{
							ID:            "field1",
							PreviousValue: tagToMap(cmsintegrationcommon.ConvertionStatusRunning),
							CurrentValue:  tagToMap(cmsintegrationcommon.ConvertionStatusNotStarted),
						},
					},
				},
			},
			fieldKey: "conv_status",
			expected: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := hasStatusChangeToNonRunning(tt.payload, tt.fieldKey)
			assert.Equal(t, tt.expected, result)
		})
	}
}

func strPtr(s string) *string {
	return &s
}
