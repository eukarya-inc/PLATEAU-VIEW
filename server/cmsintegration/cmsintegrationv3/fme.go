package cmsintegrationv3

import (
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"

	"github.com/reearth/reearthx/log"
)

type fmeRequestType string

const (
	fmeTypeQC     fmeRequestType = "qc"
	fmeTypeConv   fmeRequestType = "conv"
	fmeTypeQcConv fmeRequestType = "qc_conv"
)

func (t fmeRequestType) Title() string {
	switch t {
	case fmeTypeConv:
		return "変換"
	case fmeTypeQC:
		return "品質検査"
	default:
		return "品質検査・変換"
	}
}

var ErrInvalidFMEID = errors.New("invalid fme id")

type fmeInterface interface {
	Request(ctx context.Context, r fmeRequest) error
}

type fme struct {
	url       string
	resultURL string
	client    *http.Client
}

func newFME(url, resultURL string) *fme {
	if url == "" || resultURL == "" {
		return nil
	}
	return &fme{
		url:       url,
		resultURL: resultURL,
		client:    http.DefaultClient,
	}
}

func (s *fme) Request(ctx context.Context, r fmeRequest) error {
	b, err := json.Marshal(r)
	if err != nil {
		log.Errorfc(ctx, "fme: failed to marshal request: %v", err)
		return fmt.Errorf("failed to marshal request: %w", err)
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, s.url, strings.NewReader(string(b)))
	if err != nil {
		log.Errorfc(ctx, "fme: failed to init request: %v", err)
		return fmt.Errorf("failed to init request: %w", err)
	}

	log.Infofc(ctx, "fme: request: %s %s (id=%s, type=%s)", req.Method, req.URL.String(), r.ID, r.Type)
	log.Debugfc(ctx, "fme: request body: %s", b)

	req.Header.Set("Content-Type", "application/json")
	res, err := s.client.Do(req)
	if err != nil {
		log.Errorfc(ctx, "fme: failed to send request: url=%s, err=%v", s.url, err)
		return fmt.Errorf("failed to send: %w", err)
	}
	defer func() {
		_ = res.Body.Close()
	}()

	// 成功の場合もレスポンスボディを読んでログに出力
	body, err := io.ReadAll(res.Body)
	if err != nil {
		log.Errorfc(ctx, "fme: failed to read response body: %v", err)
		return fmt.Errorf("failed to read body: %w", err)
	}

	log.Infofc(ctx, "fme: response: status=%d", res.StatusCode)
	log.Debugfc(ctx, "fme: response body: %s", body)

	if res.StatusCode >= 300 {
		log.Errorfc(ctx, "fme: request failed: status=%d, body=%s", res.StatusCode, body)
		return fmt.Errorf("failed to request: code=%d, body=%s", res.StatusCode, body)
	}

	return nil
}

type fmeRequest struct {
	ID   string         `json:"id"`
	Type fmeRequestType `json:"type"`
	// 処理対象のzipファイルのURL
	Target string `json:"target,omitempty"`
	// 処理対象のzipファイル名
	TargetName string `json:"targetName,omitempty"`
	// JGD2011平面直角座標第1～19系のEPSGコード（6669〜6687）
	PRCS string `json:"prcs,omitempty"`
	// 品質検査パラメータファイル
	Config string `json:"config,omitempty"`
	// コードリスト
	Codelists string `json:"codelists,omitempty"`
	// スキーマ
	Schemas string `json:"schemas,omitempty"`
	// 結果を返す先のURL
	ResultURL string `json:"resultUrl,omitempty"`
	// オブジェクトリスト
	ObjectLists string `json:"objectLists,omitempty"`
}

func signFMEID(payload, secret string) string {
	mac := hmac.New(sha256.New, []byte(secret))
	_, _ = mac.Write([]byte(payload))
	sig := hex.EncodeToString(mac.Sum(nil))
	return fmt.Sprintf("%s:%s", sig, payload)
}

func unsignFMEID(id, secret string) (string, error) {
	_, payload, found := strings.Cut(id, ":")
	if !found {
		return "", ErrInvalidFMEID
	}

	if id != signFMEID(payload, secret) {
		return "", ErrInvalidFMEID
	}

	return payload, nil
}

type fmeMock struct {
	called []fmeRequest
	err    error
}

func (f *fmeMock) Request(ctx context.Context, r fmeRequest) error {
	f.called = append(f.called, r)
	d, err := json.Marshal(r)
	if err != nil {
		return err
	}
	fmt.Printf("fmeMock: %s\n", string(d))
	return f.err
}

func (f *fmeMock) Called() []fmeRequest {
	return f.called
}
