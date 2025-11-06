package policy

import (
	"context"

	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/internal/usecase/gateway"
)

type PermissiveChecker struct{}

func NewPermissiveChecker() gateway.PolicyChecker {
	return &PermissiveChecker{}
}

func (p *PermissiveChecker) CheckPolicy(_ context.Context, req gateway.PolicyCheckRequest) (*gateway.PolicyCheckResponse, error) {
	return &gateway.PolicyCheckResponse{
		Allowed:      true,
		CheckType:    req.CheckType,
		CurrentLimit: "unlimited",
		Message:      "No restrictions in OSS version",
		Value:        req.Value,
	}, nil
}
