package main

import (
	"net/http"

	"github.com/labstack/echo/v4"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/log"
)

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

		ctx := c.Request().Context()
		for i, h := range handlers {
			// Never abort the chain: a failing handler must not prevent the
			// remaining handlers from processing the same event.
			if err := h(c.Request(), w); err != nil {
				log.Errorfc(ctx, "webhook: handler %d failed: type=%s, project=%s, err=%v", i, w.Type, w.ProjectID(), err)
			}
		}

		return nil
	}, m)
}
