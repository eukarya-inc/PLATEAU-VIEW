package cmsintsetup

import (
	"net/http"

	"github.com/k0kubun/pp/v3"
	"github.com/labstack/echo/v4"
	"github.com/reearth/reearthx/log"
)

func init() {
	pp.ColoringEnabled = false
}

type Config struct {
	Token            string
	CMSURL           string
	CMSToken         string
	CMSSystemProject string
}

func Handler(conf Config, g *echo.Group) error {
	s, err := NewServices(conf)
	if err != nil {
		return err
	}

	auth := func(next echo.HandlerFunc) echo.HandlerFunc {
		return func(c echo.Context) error {
			if c.Request().Header.Get("Authorization") != "Bearer "+conf.Token {
				return c.JSON(http.StatusUnauthorized, "invalid token")
			}
			return next(c)
		}
	}

	g.Use(auth)

	g.POST("/setup", func(c echo.Context) error {
		ctx := c.Request().Context()
		ctx = log.WithPrefixMessage(ctx, "cmsintsetup: setup: ")

		var i SetupConfig
		if err := c.Bind(&i); err != nil {
			return err
		}

		log.Infofc(ctx, "received: %#v", i)

		if err := SetupModels(ctx, s, i); err != nil {
			log.Errorfc(ctx, "error: %v", err)
			return c.JSON(http.StatusBadGateway, err.Error())
		}

		return c.JSON(http.StatusOK, "ok")
	})

	g.POST("/setup_cities", func(c echo.Context) error {
		ctx := c.Request().Context()
		ctx = log.WithPrefixMessage(ctx, "cmsintsetup: setup_cities: ")

		var i SetupCityItemsInput
		if err := c.Bind(&i); err != nil {
			return err
		}

		log.Infofc(ctx, "received: %#v", i)

		if err := SetupCityItems(ctx, s, i, nil); err != nil {
			log.Errorfc(ctx, "error: %v", err)
			return c.JSON(http.StatusBadGateway, err.Error())
		}

		return c.JSON(http.StatusOK, "ok")
	})

	g.POST("/copy_related", func(c echo.Context) error {
		ctx := c.Request().Context()
		ctx = log.WithPrefixMessage(ctx, "cmsintsetup: copy_related: ")

		var i CopyRelatedItemsOpts
		if err := c.Bind(&i); err != nil {
			return err
		}

		log.Infofc(ctx, "received: %#v", i)

		if err := CopyRelatedDatasetItems(ctx, s, i); err != nil {
			log.Errorfc(ctx, "error: %v", err)
			return c.JSON(http.StatusBadGateway, err.Error())
		}

		return c.JSON(http.StatusOK, "ok")
	})

	return nil
}
