package cmsintflow

import (
	"fmt"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	cms "github.com/reearth/reearth-cms-api/go"
)

// Phase represents the lifecycle phase of a single operation (QC or Conv).
type Phase string

const (
	PhaseIdle      Phase = "idle"      // not started (nil, 未実行)
	PhaseRunning   Phase = "running"   // currently executing (実行中)
	PhaseSucceeded Phase = "succeeded" // completed successfully (成功)
	PhaseFailed    Phase = "failed"    // completed with error (エラー)
)

// WorkflowState is the compound state of (qc_status, conv_status).
type WorkflowState struct {
	QC   Phase
	Conv Phase
}

// WorkflowConfig captures static configuration that affects transitions.
type WorkflowConfig struct {
	SkipQC      bool // QC should be skipped (from IsQCAndConvSkipped or !featureType.QC)
	SkipConv    bool // Conv should be skipped (from IsQCAndConvSkipped or !featureType.Conv)
	FeatureConv bool // feature type supports Conv
}

// EventKind identifies what happened.
type EventKind int

const (
	EventWebhookReceived EventKind = iota // CMS webhook fired
	EventQCCompleted                      // QC job finished (check QCOK)
	EventConvCompleted                    // Conv job finished
	EventFlowFailed                       // Flow reported a failure
)

// Event represents something that happened in the system.
type Event struct {
	Kind    EventKind
	QCOK    bool   // only for EventQCCompleted: true if QC passed without errors
	Message string // for EventFlowFailed: error message
}

// ActionKind identifies a side-effect to be executed by the caller.
type ActionKind int

const (
	ActionSkip      ActionKind = iota // do nothing
	ActionSetStatus                   // update CMS item status fields
	ActionStartQC                     // trigger QC flow job
	ActionStartConv                   // trigger Conv flow job
	ActionFail                        // set error status + comment
)

// Action represents a side-effect to be executed by the caller.
// The caller interprets these to perform I/O (CMS updates, Flow API calls).
type Action struct {
	Kind       ActionKind
	QCStatus   cmsintegrationcommon.ConvertionStatus // for ActionSetStatus
	ConvStatus cmsintegrationcommon.ConvertionStatus // for ActionSetStatus
	Message    string                                // for ActionFail
}

// WorkflowMachine is a pure state machine for the QC/Conv workflow.
// It contains no I/O — the caller executes the returned actions.
type WorkflowMachine struct {
	State  WorkflowState
	Config WorkflowConfig
}

// NewWorkflowMachine builds the state machine from the current CMS item state.
func NewWorkflowMachine(item *cmsintegrationcommon.FeatureItem, featureQC, featureConv bool) *WorkflowMachine {
	skipQC, skipConv := item.IsQCAndConvSkipped()
	return &WorkflowMachine{
		State: WorkflowState{
			QC:   phaseFromTag(item.QCStatus),
			Conv: phaseFromTag(item.ConvertionStatus),
		},
		Config: WorkflowConfig{
			SkipQC:      skipQC || !featureQC,
			SkipConv:    skipConv || !featureConv,
			FeatureConv: featureConv,
		},
	}
}

// Transition computes the next state and actions to execute.
func (m *WorkflowMachine) Transition(event Event) ([]Action, error) {
	switch event.Kind {
	case EventWebhookReceived:
		return m.onWebhookReceived()
	case EventQCCompleted:
		return m.onQCCompleted(event.QCOK)
	case EventConvCompleted:
		return m.onConvCompleted()
	case EventFlowFailed:
		return m.onFlowFailed(event.Message)
	default:
		return nil, fmt.Errorf("unknown event kind: %d", event.Kind)
	}
}

func (m *WorkflowMachine) onWebhookReceived() ([]Action, error) {
	// Both skipped → nothing to do
	if m.Config.SkipQC && m.Config.SkipConv {
		return []Action{{Kind: ActionSkip}}, nil
	}

	// QC path: start QC (conv will be triggered after QC success)
	if !m.Config.SkipQC {
		// If QC is already past idle, skip (idempotency)
		if m.State.QC != PhaseIdle {
			return []Action{{Kind: ActionSkip}}, nil
		}

		setStatus := Action{
			Kind:     ActionSetStatus,
			QCStatus: cmsintegrationcommon.ConvertionStatusRunning,
		}
		// If conv is also supported, reset conv status to not-started
		if m.Config.FeatureConv && !m.Config.SkipConv {
			setStatus.ConvStatus = cmsintegrationcommon.ConvertionStatusNotStarted
		}

		m.State.QC = PhaseRunning
		return []Action{setStatus, {Kind: ActionStartQC}}, nil
	}

	// Conv-only path
	if !m.Config.SkipConv {
		// Guard: if QC is currently running, skip (wait for QC result callback)
		if m.State.QC == PhaseRunning {
			return []Action{{Kind: ActionSkip}}, nil
		}
		// If conv is already past idle, skip (idempotency)
		if m.State.Conv != PhaseIdle {
			return []Action{{Kind: ActionSkip}}, nil
		}

		m.State.Conv = PhaseRunning
		return []Action{
			{Kind: ActionSetStatus, ConvStatus: cmsintegrationcommon.ConvertionStatusRunning},
			{Kind: ActionStartConv},
		}, nil
	}

	return []Action{{Kind: ActionSkip}}, nil
}

func (m *WorkflowMachine) onQCCompleted(qcOK bool) ([]Action, error) {
	if !qcOK {
		m.State.QC = PhaseFailed
		return []Action{
			{Kind: ActionSetStatus, QCStatus: cmsintegrationcommon.ConvertionStatusError},
		}, nil
	}

	m.State.QC = PhaseSucceeded
	actions := []Action{
		{Kind: ActionSetStatus, QCStatus: cmsintegrationcommon.ConvertionStatusSuccess},
	}

	// Chain: if conv is enabled and not skipped, trigger conv
	if m.Config.FeatureConv && !m.Config.SkipConv {
		m.State.Conv = PhaseRunning
		actions = append(actions, Action{Kind: ActionStartConv})
	}

	return actions, nil
}

func (m *WorkflowMachine) onConvCompleted() ([]Action, error) {
	m.State.Conv = PhaseSucceeded
	return []Action{
		{Kind: ActionSetStatus, ConvStatus: cmsintegrationcommon.ConvertionStatusSuccess},
	}, nil
}

func (m *WorkflowMachine) onFlowFailed(message string) ([]Action, error) {
	if m.State.QC == PhaseRunning {
		m.State.QC = PhaseFailed
	}
	if m.State.Conv == PhaseRunning {
		m.State.Conv = PhaseFailed
	}
	return []Action{
		{Kind: ActionFail, Message: message},
	}, nil
}

// phaseFromTag maps a CMS tag value to a Phase.
func phaseFromTag(tag *cms.Tag) Phase {
	if tag == nil {
		return PhaseIdle
	}
	switch cmsintegrationcommon.ConvertionStatus(tag.Name) {
	case cmsintegrationcommon.ConvertionStatusRunning:
		return PhaseRunning
	case cmsintegrationcommon.ConvertionStatusSuccess:
		return PhaseSucceeded
	case cmsintegrationcommon.ConvertionStatusError:
		return PhaseFailed
	default:
		return PhaseIdle // 未実行 or unknown
	}
}

// ReqTypeForAction returns the ReqType that should be used when executing an action.
// This bridges the state machine's action output to the existing Flow trigger API.
func ReqTypeForAction(a Action) cmsintegrationcommon.ReqType {
	switch a.Kind {
	case ActionStartQC:
		return cmsintegrationcommon.ReqTypeQC
	case ActionStartConv:
		return cmsintegrationcommon.ReqTypeConv
	default:
		return ""
	}
}
