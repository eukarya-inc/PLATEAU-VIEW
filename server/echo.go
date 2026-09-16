package main

import (
	"context"
	"net/http"
	"time"

	"github.com/labstack/echo/v4"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/log"
)

// webhookHandlerTimeout bounds the handler chain once it is detached from the
// request context. It matches the Cloud Run request timeout configured for this
// service, which is what used to bound the chain in practice.
const webhookHandlerTimeout = time.Hour

func cmsWebhookHandler(g *echo.Group, secret []byte, handlers []cmswebhook.Handler) {
	m := echo.WrapMiddleware(cmswebhook.Middleware(cmswebhook.MiddlewareConfig{
		Secret: secret,
		Logger: log.Debugfc,
	}))

	g.GET("/ping", func(c echo.Context) error {
		return c.String(http.StatusOK, "pong")
	})

	g.POST("/ping", func(c echo.Context) error {
		jsonMap := make(map[string]any)
		if err := c.Bind(&jsonMap); err == nil {
			log.Debugfc(c.Request().Context(), "ping json: %v", jsonMap)
		}
		return c.String(http.StatusOK, "pong")
	})

	g.POST("", func(c echo.Context) error {
		w := cmswebhook.GetPayload(c.Request().Context())
		if w == nil {
			return c.JSON(http.StatusUnauthorized, map[string]string{"error": "unauthorized"})
		}

		// Respond before running the handlers: some of them talk to slow external
		// services (e.g. G空間情報センター) and the CMS would time out otherwise.
		// Therefore handler failures cannot be reported to the CMS and have to be
		// logged loudly instead.
		if err := c.JSON(http.StatusOK, "ok"); err != nil {
			return err
		}

		// Echo's response writer is buffered, so without an explicit flush the 200
		// above would not leave the process until this handler returns and the CMS
		// would block on the whole chain anyway.
		c.Response().Flush()

		// The handlers keep running after the response was sent, so they must not
		// use the request context: once the CMS client gives up and disconnects,
		// net/http cancels it and an in-flight publication would be torn down
		// halfway through.
		ctx, cancel := context.WithTimeout(context.WithoutCancel(c.Request().Context()), webhookHandlerTimeout)
		defer cancel()
		req := c.Request().WithContext(ctx)

		for i, h := range handlers {
			// Never abort the chain: a failing handler must not prevent the
			// remaining handlers from processing the same event.
			if err := h(req, w); err != nil {
				log.Errorfc(ctx, "webhook: handler %d failed: type=%s, project=%s, err=%v", i, w.Type, w.ProjectID(), err)
			}
		}

		return nil
	}, m)
}
