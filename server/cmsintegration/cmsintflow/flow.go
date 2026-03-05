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
	Cancel(ctx context.Context, baseURL, runID, triggerID string) error
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

	// Use request's BaseURL if provided, otherwise use client's default
	baseURL := f.baseURL
	if r.BaseURL != "" {
		baseURL = r.BaseURL
	}

	u := getTriggerURL(baseURL, r.TriggerID)
	if u == "" {
		log.Errorfc(ctx, "invalid flow url: base_url=%s, trigger_id=%s", baseURL, r.TriggerID)
		return FlowRequestResult{}, fmt.Errorf("invalid url: base_url=%s, trigger_id=%s", baseURL, r.TriggerID)
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

func (f *flowImpl) Cancel(ctx context.Context, baseURL, runID, triggerID string) error {
	if baseURL == "" {
		baseURL = f.baseURL
	}

	if baseURL == "" || runID == "" || triggerID == "" {
		return fmt.Errorf("invalid cancel request: baseURL=%s, runID=%s, triggerID=%s", baseURL, runID, triggerID)
	}

	u, err := url.JoinPath(baseURL, "api", "jobs", runID, "cancel")
	if err != nil {
		return fmt.Errorf("failed to build cancel URL: %w", err)
	}

	body := map[string]string{
		"triggerId": triggerID,
		"authToken": f.token,
	}
	b, err := json.Marshal(body)
	if err != nil {
		return fmt.Errorf("failed to marshal cancel request: %w", err)
	}

	req, err := http.NewRequestWithContext(ctx, "POST", u, bytes.NewReader(b))
	if err != nil {
		log.Errorfc(ctx, "failed to create cancel request: %v", err)
		return fmt.Errorf("failed to create cancel request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+f.token)

	log.Infofc(ctx, "flow cancel: url=%s, runID=%s, triggerID=%s", u, runID, triggerID)

	resp, err := f.h.Do(req)
	if err != nil {
		log.Errorfc(ctx, "failed to send cancel request: url=%s, err=%v", u, err)
		return fmt.Errorf("failed to send cancel request: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()

	resb, err := io.ReadAll(resp.Body)
	if err != nil {
		log.Errorfc(ctx, "failed to read cancel response: %v", err)
		return fmt.Errorf("failed to read cancel response: %w", err)
	}

	log.Infofc(ctx, "flow cancel resp: status=%s", resp.Status)
	log.Debugfc(ctx, "flow cancel resp body: %s", resb)

	if resp.StatusCode != http.StatusOK {
		log.Errorfc(ctx, "cancel request failed: status=%d, body=%s", resp.StatusCode, resb)
		return fmt.Errorf("cancel failed: status=%d, body=%s", resp.StatusCode, resb)
	}

	return nil
}

func getTriggerURL(baseURL, triggerID string) string {
	if baseURL == "" {
		if _, err := url.Parse(triggerID); err == nil {
			return triggerID
		}

		return ""
	}

	u, _ := url.JoinPath(baseURL, "api", "triggers", triggerID, "run")
	return u
}
