package cmsintegrationv3

import (
	"net/http"

	"github.com/labstack/echo/v4"
	"github.com/reearth/reearthx/log"
)

func Handler(conf Config, g *echo.Group) error {
	s, err := NewServices(conf)
	if err != nil {
		return err
	}

	g.POST(fmeHandlerPath, func(c echo.Context) error {
		ctx := c.Request().Context()

		var f fmeResult
		if err := c.Bind(&f); err != nil {
			log.Infofc(ctx, "cmsintegrationv3 notify: invalid payload: %w", err)
			return c.JSON(http.StatusBadRequest, "invalid payload")
		}

		log.Infofc(ctx, "cmsintegrationv3 notify: received: %#v", f)

		if err := receiveResultFromFME(ctx, s, &conf, f); err != nil {
			log.Errorfc(ctx, "cmsintegrationv3 notify: failed to receive result from fme: %v", err)
			return c.JSON(http.StatusInternalServerError, "failed to receive result from fme")
		}

		return nil
	})

	return nil
}
