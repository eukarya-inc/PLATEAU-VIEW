package http

import (
	"context"

	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/worker/internal/usecase/interactor"
	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/worker/pkg/webhook"
)

type WebhookController struct {
	usecase *interactor.Usecase
}

func NewWebhookController(u *interactor.Usecase) *WebhookController {
	return &WebhookController{
		usecase: u,
	}
}

func (c *WebhookController) Webhook(ctx context.Context, w *webhook.Webhook) error {
	return c.usecase.SendWebhook(ctx, w)
}
