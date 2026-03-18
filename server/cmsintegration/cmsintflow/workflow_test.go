package cmsintflow

import (
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/stretchr/testify/assert"
)

func TestPhaseFromTag(t *testing.T) {
	tests := []struct {
		name string
		tag  *cms.Tag
		want Phase
	}{
		{"nil", nil, PhaseIdle},
		{"未実行", &cms.Tag{Name: "未実行"}, PhaseIdle},
		{"実行中", &cms.Tag{Name: "実行中"}, PhaseRunning},
		{"成功", &cms.Tag{Name: "成功"}, PhaseSucceeded},
		{"エラー", &cms.Tag{Name: "エラー"}, PhaseFailed},
		{"unknown", &cms.Tag{Name: "unknown"}, PhaseIdle},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			assert.Equal(t, tt.want, phaseFromTag(tt.tag))
		})
	}
}

func TestNewWorkflowMachine(t *testing.T) {
	tests := []struct {
		name        string
		item        *cmsintegrationcommon.FeatureItem
		featureQC   bool
		featureConv bool
		wantState   WorkflowState
		wantConfig  WorkflowConfig
	}{
		{
			name:        "fresh item, both supported",
			item:        &cmsintegrationcommon.FeatureItem{},
			featureQC:   true,
			featureConv: true,
			wantState:   WorkflowState{QC: PhaseIdle, Conv: PhaseIdle},
			wantConfig:  WorkflowConfig{SkipQC: false, SkipConv: false, FeatureConv: true},
		},
		{
			name:        "conv only feature type",
			item:        &cmsintegrationcommon.FeatureItem{},
			featureQC:   false,
			featureConv: true,
			wantState:   WorkflowState{QC: PhaseIdle, Conv: PhaseIdle},
			wantConfig:  WorkflowConfig{SkipQC: true, SkipConv: false, FeatureConv: true},
		},
		{
			name: "変換のみ実行",
			item: &cmsintegrationcommon.FeatureItem{
				SkipQCConv: &cms.Tag{Name: "変換のみ実行"},
			},
			featureQC:   true,
			featureConv: true,
			wantState:   WorkflowState{QC: PhaseIdle, Conv: PhaseIdle},
			wantConfig:  WorkflowConfig{SkipQC: true, SkipConv: false, FeatureConv: true},
		},
		{
			name: "QC running",
			item: &cmsintegrationcommon.FeatureItem{
				QCStatus: &cms.Tag{Name: "実行中"},
			},
			featureQC:   true,
			featureConv: true,
			wantState:   WorkflowState{QC: PhaseRunning, Conv: PhaseIdle},
			wantConfig:  WorkflowConfig{SkipQC: true, SkipConv: false, FeatureConv: true},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			wm := NewWorkflowMachine(tt.item, tt.featureQC, tt.featureConv)
			assert.Equal(t, tt.wantState, wm.State)
			assert.Equal(t, tt.wantConfig, wm.Config)
		})
	}
}

func TestWorkflowMachine_WebhookReceived(t *testing.T) {
	tests := []struct {
		name        string
		state       WorkflowState
		config      WorkflowConfig
		wantActions []Action
		wantState   WorkflowState
	}{
		{
			name:        "both skipped",
			state:       WorkflowState{QC: PhaseIdle, Conv: PhaseIdle},
			config:      WorkflowConfig{SkipQC: true, SkipConv: true},
			wantActions: []Action{{Kind: ActionSkip}},
			wantState:   WorkflowState{QC: PhaseIdle, Conv: PhaseIdle},
		},
		{
			name:   "start QC (both supported)",
			state:  WorkflowState{QC: PhaseIdle, Conv: PhaseIdle},
			config: WorkflowConfig{SkipQC: false, SkipConv: false, FeatureConv: true},
			wantActions: []Action{
				{Kind: ActionSetStatus, QCStatus: cmsintegrationcommon.ConvertionStatusRunning, ConvStatus: cmsintegrationcommon.ConvertionStatusNotStarted},
				{Kind: ActionStartQC},
			},
			wantState: WorkflowState{QC: PhaseRunning, Conv: PhaseIdle},
		},
		{
			name:   "start QC (QC only, no conv)",
			state:  WorkflowState{QC: PhaseIdle, Conv: PhaseIdle},
			config: WorkflowConfig{SkipQC: false, SkipConv: true, FeatureConv: false},
			wantActions: []Action{
				{Kind: ActionSetStatus, QCStatus: cmsintegrationcommon.ConvertionStatusRunning},
				{Kind: ActionStartQC},
			},
			wantState: WorkflowState{QC: PhaseRunning, Conv: PhaseIdle},
		},
		{
			name:   "start conv only (skipQC)",
			state:  WorkflowState{QC: PhaseIdle, Conv: PhaseIdle},
			config: WorkflowConfig{SkipQC: true, SkipConv: false, FeatureConv: true},
			wantActions: []Action{
				{Kind: ActionSetStatus, ConvStatus: cmsintegrationcommon.ConvertionStatusRunning},
				{Kind: ActionStartConv},
			},
			wantState: WorkflowState{QC: PhaseIdle, Conv: PhaseRunning},
		},
		{
			name:        "conv only but QC is running → skip (guard)",
			state:       WorkflowState{QC: PhaseRunning, Conv: PhaseIdle},
			config:      WorkflowConfig{SkipQC: true, SkipConv: false, FeatureConv: true},
			wantActions: []Action{{Kind: ActionSkip}},
			wantState:   WorkflowState{QC: PhaseRunning, Conv: PhaseIdle},
		},
		{
			name:        "QC already running → skip (idempotency)",
			state:       WorkflowState{QC: PhaseRunning, Conv: PhaseIdle},
			config:      WorkflowConfig{SkipQC: false, SkipConv: false, FeatureConv: true},
			wantActions: []Action{{Kind: ActionSkip}},
			wantState:   WorkflowState{QC: PhaseRunning, Conv: PhaseIdle},
		},
		{
			name:        "QC succeeded, conv not started → skip (idempotency, QC won't re-run)",
			state:       WorkflowState{QC: PhaseSucceeded, Conv: PhaseIdle},
			config:      WorkflowConfig{SkipQC: false, SkipConv: false, FeatureConv: true},
			wantActions: []Action{{Kind: ActionSkip}},
			wantState:   WorkflowState{QC: PhaseSucceeded, Conv: PhaseIdle},
		},
		{
			name:        "both succeeded → skip",
			state:       WorkflowState{QC: PhaseSucceeded, Conv: PhaseSucceeded},
			config:      WorkflowConfig{SkipQC: true, SkipConv: true},
			wantActions: []Action{{Kind: ActionSkip}},
			wantState:   WorkflowState{QC: PhaseSucceeded, Conv: PhaseSucceeded},
		},
		{
			name:        "conv already running → skip (idempotency)",
			state:       WorkflowState{QC: PhaseIdle, Conv: PhaseRunning},
			config:      WorkflowConfig{SkipQC: true, SkipConv: false, FeatureConv: true},
			wantActions: []Action{{Kind: ActionSkip}},
			wantState:   WorkflowState{QC: PhaseIdle, Conv: PhaseRunning},
		},
		{
			name:   "QC errored, conv not started, skipQC → start conv",
			state:  WorkflowState{QC: PhaseFailed, Conv: PhaseIdle},
			config: WorkflowConfig{SkipQC: true, SkipConv: false, FeatureConv: true},
			wantActions: []Action{
				{Kind: ActionSetStatus, ConvStatus: cmsintegrationcommon.ConvertionStatusRunning},
				{Kind: ActionStartConv},
			},
			wantState: WorkflowState{QC: PhaseFailed, Conv: PhaseRunning},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			wm := &WorkflowMachine{State: tt.state, Config: tt.config}
			actions, err := wm.Transition(Event{Kind: EventWebhookReceived})
			assert.NoError(t, err)
			assert.Equal(t, tt.wantActions, actions)
			assert.Equal(t, tt.wantState, wm.State)
		})
	}
}

func TestWorkflowMachine_QCCompleted(t *testing.T) {
	tests := []struct {
		name        string
		state       WorkflowState
		config      WorkflowConfig
		qcOK        bool
		wantActions []Action
		wantState   WorkflowState
	}{
		{
			name:   "QC ok, conv enabled → trigger conv",
			state:  WorkflowState{QC: PhaseRunning, Conv: PhaseIdle},
			config: WorkflowConfig{SkipConv: false, FeatureConv: true},
			qcOK:   true,
			wantActions: []Action{
				{Kind: ActionSetStatus, QCStatus: cmsintegrationcommon.ConvertionStatusSuccess},
				{Kind: ActionStartConv},
			},
			wantState: WorkflowState{QC: PhaseSucceeded, Conv: PhaseRunning},
		},
		{
			name:   "QC ok, conv skipped → no conv",
			state:  WorkflowState{QC: PhaseRunning, Conv: PhaseIdle},
			config: WorkflowConfig{SkipConv: true, FeatureConv: true},
			qcOK:   true,
			wantActions: []Action{
				{Kind: ActionSetStatus, QCStatus: cmsintegrationcommon.ConvertionStatusSuccess},
			},
			wantState: WorkflowState{QC: PhaseSucceeded, Conv: PhaseIdle},
		},
		{
			name:   "QC ok, feature has no conv → no conv",
			state:  WorkflowState{QC: PhaseRunning, Conv: PhaseIdle},
			config: WorkflowConfig{SkipConv: false, FeatureConv: false},
			qcOK:   true,
			wantActions: []Action{
				{Kind: ActionSetStatus, QCStatus: cmsintegrationcommon.ConvertionStatusSuccess},
			},
			wantState: WorkflowState{QC: PhaseSucceeded, Conv: PhaseIdle},
		},
		{
			name:   "QC not ok → error, no conv",
			state:  WorkflowState{QC: PhaseRunning, Conv: PhaseIdle},
			config: WorkflowConfig{SkipConv: false, FeatureConv: true},
			qcOK:   false,
			wantActions: []Action{
				{Kind: ActionSetStatus, QCStatus: cmsintegrationcommon.ConvertionStatusError},
			},
			wantState: WorkflowState{QC: PhaseFailed, Conv: PhaseIdle},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			wm := &WorkflowMachine{State: tt.state, Config: tt.config}
			actions, err := wm.Transition(Event{Kind: EventQCCompleted, QCOK: tt.qcOK})
			assert.NoError(t, err)
			assert.Equal(t, tt.wantActions, actions)
			assert.Equal(t, tt.wantState, wm.State)
		})
	}
}

func TestWorkflowMachine_ConvCompleted(t *testing.T) {
	wm := &WorkflowMachine{
		State:  WorkflowState{QC: PhaseSucceeded, Conv: PhaseRunning},
		Config: WorkflowConfig{FeatureConv: true},
	}
	actions, err := wm.Transition(Event{Kind: EventConvCompleted})
	assert.NoError(t, err)
	assert.Equal(t, []Action{
		{Kind: ActionSetStatus, ConvStatus: cmsintegrationcommon.ConvertionStatusSuccess},
	}, actions)
	assert.Equal(t, WorkflowState{QC: PhaseSucceeded, Conv: PhaseSucceeded}, wm.State)
}

func TestWorkflowMachine_FlowFailed(t *testing.T) {
	tests := []struct {
		name      string
		state     WorkflowState
		wantState WorkflowState
	}{
		{
			name:      "QC running → QC failed",
			state:     WorkflowState{QC: PhaseRunning, Conv: PhaseIdle},
			wantState: WorkflowState{QC: PhaseFailed, Conv: PhaseIdle},
		},
		{
			name:      "Conv running → Conv failed",
			state:     WorkflowState{QC: PhaseSucceeded, Conv: PhaseRunning},
			wantState: WorkflowState{QC: PhaseSucceeded, Conv: PhaseFailed},
		},
		{
			name:      "QC succeeded, Conv running → Conv failed",
			state:     WorkflowState{QC: PhaseIdle, Conv: PhaseRunning},
			wantState: WorkflowState{QC: PhaseIdle, Conv: PhaseFailed},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			wm := &WorkflowMachine{State: tt.state, Config: WorkflowConfig{}}
			actions, err := wm.Transition(Event{Kind: EventFlowFailed, Message: "connection error"})
			assert.NoError(t, err)
			assert.Equal(t, []Action{
				{Kind: ActionFail, Message: "connection error"},
			}, actions)
			assert.Equal(t, tt.wantState, wm.State)
		})
	}
}

func TestReqTypeForAction(t *testing.T) {
	assert.Equal(t, cmsintegrationcommon.ReqTypeQC, ReqTypeForAction(Action{Kind: ActionStartQC}))
	assert.Equal(t, cmsintegrationcommon.ReqTypeConv, ReqTypeForAction(Action{Kind: ActionStartConv}))
	assert.Equal(t, cmsintegrationcommon.ReqType(""), ReqTypeForAction(Action{Kind: ActionSkip}))
}
