# cmsintflow: QC/Conv State Machine

## Overview

This package handles the QC (Quality Check) and Conversion workflow for PLATEAU CityGML data via FME Flow integration. The workflow is driven by CMS webhooks and Flow result callbacks.

## Entry Points

1. **Webhook** (`handler.go`): CMS fires `item.update` webhook on any item change → `sendRequestToFlow()`
2. **Flow Result** (`res.go`): Flow sends result callback → `receiveResultFromFlow()`

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
| `品質検査・変換のみをスキップ` | Skip both | **true** | **true** |

## State Machine

### States

Each item has a compound state of `(qc_status, conv_status)`. Key states:

```
INIT        = (nil,    nil)      # Item just created
READY       = (未実行, 未実行)   # Statuses explicitly set to not-started
QC_RUNNING  = (実行中, 未実行)   # QC in progress
QC_OK       = (成功,   未実行)   # QC succeeded, conv not yet started
QC_ERR      = (エラー, 未実行)   # QC failed
CONV_RUNNING= (成功,   実行中)   # Conv in progress (after QC success)
DONE        = (成功,   成功)     # Both completed successfully
CONV_ERR    = (成功,   エラー)   # Conv failed
```

### Type Determination Logic

```
IsQCAndConvSkipped(item) → (skipQC, skipConv)
  1. Status-based:
     - qc_status ∉ {nil, 未実行} → skipQC = true
     - conv_status ∉ {nil, 未実行} → skipConv = true
     - Both true → return early
  2. skip_qc_conv tag:
     - Contains "スキップ": skip what's mentioned
     - Contains "実行": run what's mentioned, skip the rest
  3. Legacy bool fields (SkipQC, SkipConvert)

ReqType = ReqTypeFrom(skipQC, skipConv)
  - skipQC=T, skipConv=T → "" (nothing to do)
  - skipQC=T, skipConv=F → "conv"
  - skipQC=F, skipConv=T → "qc"
  - skipQC=F, skipConv=F → "qc_conv"

sendRequestToFlow:
  fty = feature type capability (qc_conv if both supported)
  ity = item.ReqType().Override(overrideReqType)
  ty  = fty.Intersection(ity).Normalize()
        Normalize: qc_conv → qc (QC runs first)
  Guard: ty == "conv" && qc_status == 実行中 → skip (wait for QC)
```

### Transitions

#### Normal Flow: QC + Conv (skip_qc_conv = nil or "品質検査・変換を実行")

```
                    Webhook                          Flow Result (QC OK)
  INIT ──────────────────────► QC_RUNNING ──────────────────────────► CONV_RUNNING
  (nil, nil)                   (実行中, 未実行)                       (成功, 実行中)
  skipQC=F, skipConv=F                                                    │
  ity=qc_conv → ty=qc                                                    │
  UpdateStatus(qc_conv,実行中)                                            │
    → qc=実行中, conv=未実行                                    Flow Result (Conv OK)
                                                                          │
                                                                          ▼
                                                                       DONE
                                                                    (成功, 成功)
```

QC failure path:
```
  QC_RUNNING ──── Flow Result (QC NG) ────► QC_ERR
  (実行中, 未実行)                           (エラー, 未実行)
                                             Conv is NOT triggered.
```

#### Conv Only (skip_qc_conv = "変換のみ実行")

```
                    Webhook
  INIT ──────────────────────► CONV_RUNNING ──────────► DONE
  (nil, nil)                   (nil, 実行中)             (nil, 成功)
  skipQC=T, skipConv=F
  ity=conv → ty=conv
  UpdateStatus(conv,実行中)
    → conv=実行中
```

#### Webhook Re-entry Guards

When CMS updates an item (e.g. status change), another webhook fires. The guards prevent re-processing:

| Current State | skipQC | skipConv | ity | ty | Result |
|--------------|--------|----------|-----|-----|--------|
| (実行中, 未実行) | T | F | conv | conv | **Blocked** by L74 guard (QC running) |
| (実行中, 実行中) | T | T | "" | "" | Skip (nothing to do) |
| (成功, 実行中) | T | T | "" | "" | Skip (nothing to do) |
| (成功, 成功) | T | T | "" | "" | Skip (nothing to do) |
| (エラー, 未実行) | T | F | conv | conv | Conv runs from webhook |

### receiveResultFromFlow (res.go)

```
QC Result:
  status=success && QCOK=true:
    → Update qc_status=成功
    → Check IsQCAndConvSkipped() for conv
    → If conv not skipped: sendRequestToFlow(overrideReqType=conv)
  status=success && QCOK=false:
    → Update qc_status=成功
    → Log "QC detected errors", do NOT trigger conv
  status=error:
    → Update qc_status=エラー

Conv Result:
  status=success:
    → Update conv_status=成功
    → Upload result assets
  status=error:
    → Update conv_status=エラー
```

## Key Design Decisions

1. **QC runs before Conv**: `Normalize()` converts `qc_conv` → `qc`. Conv is triggered only after QC succeeds via `receiveResultFromFlow`.
2. **Status-based idempotency**: Once a status moves past 未実行, that phase is considered "done or in-progress" and won't be re-triggered by webhooks.
3. **L74 guard**: Prevents conv from starting while QC is still running (webhook re-entry from status update).
4. **"実行" tag values**: Skip what's NOT mentioned (e.g. "変換のみ実行" skips QC).
