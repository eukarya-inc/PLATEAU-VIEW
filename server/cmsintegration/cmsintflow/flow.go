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
		return FlowRequestResult{}, fmt.Errorf("failed to marshal request: %w", err)
	}

	u := f.getTriggerURL(r.TriggerID)
	if u == "" {
		return FlowRequestResult{}, fmt.Errorf("invalid url: base_url=%s, trigger_id=%s", f.baseURL, r.TriggerID)
	}

	req, err := http.NewRequestWithContext(ctx, "POST", u, bytes.NewReader(b))
	if err != nil {
		return FlowRequestResult{}, fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Content-Type", "application/json")

	log.Debugfc(ctx, "flow req: url=%s, body=%s", u, b)
	if r.DryRun {
		log.Debugfc(ctx, "dry run")
		return
	}

	resp, err := f.h.Do(req)
	if err != nil {
		return FlowRequestResult{}, fmt.Errorf("failed to send request: %w", err)
	}

	defer resp.Body.Close()
	resb, err := io.ReadAll(resp.Body)
	if err != nil {
		return FlowRequestResult{}, fmt.Errorf("failed to read response: %w", err)
	}

	log.Debugfc(ctx, "flow resp: status=%s, body=%s", resp.Status, resb)

	if resp.StatusCode != http.StatusOK {
		return FlowRequestResult{}, fmt.Errorf("failed to send request: %s", resp.Status)
	}

	if err := json.Unmarshal(resb, &res); err != nil {
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
