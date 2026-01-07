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
		ctx = log.WithPrefixMessage(ctx, "cmsintegrationv3 notify: ")

		log.Infofc(ctx, "incoming")

		var f fmeResult
		if err := c.Bind(&f); err != nil {
			log.Warnfc(ctx, "invalid payload: %v", err)
			return c.JSON(http.StatusBadRequest, "invalid payload")
		}

		log.Infofc(ctx, "received: id=%s, status=%s, type=%s", f.ID, f.Status, f.Type)
		log.Debugfc(ctx, "received payload: %#v", f)

		if err := receiveResultFromFME(ctx, s, &conf, f); err != nil {
			log.Errorfc(ctx, "failed to receive result from fme: %v", err)
			return c.JSON(http.StatusInternalServerError, "failed to receive result from fme")
		}

		log.Infofc(ctx, "done: id=%s", f.ID)
		return nil
	})

	return nil
}
