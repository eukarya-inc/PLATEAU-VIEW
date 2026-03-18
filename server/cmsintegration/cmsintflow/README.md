# cmsintflow: QC/Conv Workflow State Machine

## Overview

This package handles the QC (Quality Check) and Conversion workflow for PLATEAU CityGML data via FME Flow integration. The workflow is driven by CMS webhooks and Flow result callbacks.

## Architecture

```
handler.go          req.go                 workflow.go              res.go
┌──────────┐    ┌───────────────┐    ┌──────────────────┐    ┌────────────────┐
│ Webhook  │───►│sendRequestTo  │───►│WorkflowMachine   │    │receiveResult   │
│          │    │Flow()         │    │ .Transition()    │    │FromFlow()      │
└──────────┘    │               │    │                  │    │                │
                │ Build SM      │    │ Pure logic:      │◄───│ Build SM       │
                │ Execute       │    │ - State          │    │ Execute        │
                │ Actions       │    │ - Config         │    │ Actions        │
                └───────────────┘    │ - Events→Actions │    └────────────────┘
                                     └──────────────────┘
```

The state machine (`workflow.go`) is **pure** — it contains no I/O. It takes the current item state and an event, and returns a list of actions. The callers (`req.go`, `res.go`) execute those actions (CMS updates, Flow API calls).

## Item Fields

| Field | CMS Key | Type | Values |
|-------|---------|------|--------|
| QC Status | `qc_status` | tag (metadata) | nil, 未実行, 実行中, 成功, エラー |
| Conv Status | `conv_status` | tag (metadata) | nil, 未実行, 実行中, 成功, エラー |
| Skip QC/Conv | `skip_qc_conv` | tag (metadata) | See below |

### skip_qc_conv Values

| Value | Meaning | skipQC | skipConv |
|-------|---------|--------|----------|
| nil (unset) | Run both QC and Conv | - | - |
| `品質検査・変換を実行` | Run both QC and Conv | false | false |
| `品質検査のみ実行` | Run QC only, skip Conv | false | **true** |
| `変換のみ実行` | Run Conv only, skip QC | **true** | false |
| `品質検査のみをスキップ` | Skip QC, run Conv | **true** | false |
| `変換のみをスキップ` | Skip Conv, run QC | false | **true** |
| `品質検査・変換をスキップ` | Skip both | **true** | **true** |

## State Machine

### Types (`workflow.go`)

```go
Phase:    idle | running | succeeded | failed
State:    (QC Phase, Conv Phase)
Config:   SkipQC, SkipConv, FeatureConv
Event:    WebhookReceived | QCCompleted(QCOK) | ConvCompleted | FlowFailed
Action:   Skip | SetStatus(qc, conv) | StartQC | StartConv | Fail(msg)
```

### Transition Table

| State (QC, Conv) | Event | Condition | Actions | New State |
|---|---|---|---|---|
| (idle, idle) | Webhook | !skipQC | SetStatus(qc=実行中, conv=未実行), StartQC | (running, idle) |
| (idle, idle) | Webhook | skipQC, !skipConv | SetStatus(conv=実行中), StartConv | (idle, running) |
| (any, any) | Webhook | skipQC && skipConv | Skip | — |
| (running, idle) | Webhook | skipQC, !skipConv | Skip (QC running guard) | — |
| (≠idle, any) | Webhook | !skipQC | Skip (idempotency) | — |
| (any, ≠idle) | Webhook | skipQC, !skipConv | Skip (idempotency) | — |
| (running, idle) | QCCompleted(ok) | !skipConv, featureConv | SetStatus(qc=成功), StartConv | (succeeded, running) |
| (running, idle) | QCCompleted(ok) | skipConv or !featureConv | SetStatus(qc=成功) | (succeeded, idle) |
| (running, idle) | QCCompleted(!ok) | — | SetStatus(qc=エラー) | (failed, idle) |
| (any, running) | ConvCompleted | — | SetStatus(conv=成功) | (any, succeeded) |
| (any, any) | FlowFailed | — | Fail(msg) | (running→failed, running→failed) |

### Normal Flow: QC + Conv

```
  (idle, idle)   ──Webhook──►  (running, idle)  ──QC OK──►  (succeeded, running)  ──Conv OK──►  (succeeded, succeeded)
                  StartQC                         StartConv                                       Done!
```

### Conv Only Flow (skip_qc_conv = "変換のみ実行")

```
  (idle, idle)   ──Webhook──►  (idle, running)  ──Conv OK──►  (idle, succeeded)
                  StartConv                                     Done!
```

### QC Failure

```
  (running, idle)  ──QC NG──►  (failed, idle)
                                Conv is NOT triggered.
```

### Flow Failure (error during QC or Conv)

```
  (running, idle)   ──FlowFailed──►  (failed, idle)
  (any, running)    ──FlowFailed──►  (any, failed)
```

## Key Design Decisions

1. **Pure state machine**: `WorkflowMachine.Transition()` has no side effects. All I/O (CMS updates, Flow API) is performed by the caller based on returned actions. This makes the state logic fully testable without mocks.
2. **QC before Conv**: When both are enabled, QC runs first. Conv is triggered by `QCCompleted(ok=true)` returning `StartConv`.
3. **Idempotency**: If a phase has already moved past `idle`, the webhook won't re-trigger it. This prevents re-entry from webhook cascades (CMS fires webhooks when status is updated).
4. **QC running guard**: When conv-only is requested but QC is still running, the webhook is skipped. Conv will be triggered by the QC result callback instead.
5. **"実行" tag values**: `IsQCAndConvSkipped()` (in `cmsintegrationcommon`) handles both "スキップ" and "実行" values in the `skip_qc_conv` tag.
