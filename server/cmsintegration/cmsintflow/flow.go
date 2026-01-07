package cmsintflow

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"

	"github.com/reearth/reearthx/log"
)

type Flow interface {
	Request(context.Context, FlowRequest) (FlowRequestResult, error)
}

type flowImpl struct {
	h       *http.Client
	baseURL string
	token   string
}

func NewFlow(h *http.Client, baseURL, token string) Flow {
	if h == nil {
		h = http.DefaultClient
	}
	return &flowImpl{h: h, baseURL: baseURL, token: token}
}

func (f *flowImpl) Request(ctx context.Context, r FlowRequest) (res FlowRequestResult, _ error) {
	b, err := json.Marshal(r)
	if err != nil {
		log.Errorfc(ctx, "failed to marshal flow request: %v", err)
		return FlowRequestResult{}, fmt.Errorf("failed to marshal request: %w", err)
	}

	u := f.getTriggerURL(r.TriggerID)
	if u == "" {
		log.Errorfc(ctx, "invalid flow url: base_url=%s, trigger_id=%s", f.baseURL, r.TriggerID)
		return FlowRequestResult{}, fmt.Errorf("invalid url: base_url=%s, trigger_id=%s", f.baseURL, r.TriggerID)
	}

	req, err := http.NewRequestWithContext(ctx, "POST", u, bytes.NewReader(b))
	if err != nil {
		log.Errorfc(ctx, "failed to create flow request: %v", err)
		return FlowRequestResult{}, fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Content-Type", "application/json")

	log.Infofc(ctx, "flow req: url=%s", u)
	log.Debugfc(ctx, "flow req body: %s", b)
	if r.DryRun {
		log.Infofc(ctx, "dry run: skipping actual request")
		return
	}

	resp, err := f.h.Do(req)
	if err != nil {
		log.Errorfc(ctx, "failed to send flow request: url=%s, err=%v", u, err)
		return FlowRequestResult{}, fmt.Errorf("failed to send request: %w", err)
	}

	defer func() { _ = resp.Body.Close() }()
	resb, err := io.ReadAll(resp.Body)
	if err != nil {
		log.Errorfc(ctx, "failed to read flow response: %v", err)
		return FlowRequestResult{}, fmt.Errorf("failed to read response: %w", err)
	}

	log.Infofc(ctx, "flow resp: status=%s", resp.Status)
	log.Debugfc(ctx, "flow resp body: %s", resb)

	if resp.StatusCode != http.StatusOK {
		log.Errorfc(ctx, "flow request failed: status=%d, body=%s", resp.StatusCode, resb)
		return FlowRequestResult{}, fmt.Errorf("failed to send request: status=%d, body=%s", resp.StatusCode, resb)
	}

	if err := json.Unmarshal(resb, &res); err != nil {
		log.Errorfc(ctx, "failed to decode flow response: body=%s, err=%v", resb, err)
		return FlowRequestResult{}, fmt.Errorf("failed to decode response: %w", err)
	}

	return
}

func (f *flowImpl) getTriggerURL(triggerID string) string {
	base := f.baseURL
	if base == "" {
		if _, err := url.Parse(triggerID); err == nil {
			return triggerID
		}

		return ""
	}

	u, _ := url.JoinPath(f.baseURL, "api", "triggers", triggerID, "run")
	return u
}
