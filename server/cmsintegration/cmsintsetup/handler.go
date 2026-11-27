package cmsintsetup

import (
	"net/http"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/serviceauth"
	"github.com/k0kubun/pp/v3"
	"github.com/labstack/echo/v4"
	"github.com/reearth/reearthx/log"
)

func init() {
	pp.ColoringEnabled = false
}

type Config struct {
	// Host is the URL of this server, the audience of the ID tokens of ServiceAccounts.
	Host string
	// ServiceAccounts may call the setup routes with Google ID tokens. The routes reject every
	// request when it is empty.
	ServiceAccounts  []string
	CMSURL           string
	CMSToken         string
	CMSSystemProject string
	// JobName is the full name of the Cloud Run Job that runs this server's image with tool commands
	// (projects/<project>/locations/<region>/jobs/<job>). City setup from the workspaces model is disabled when empty.
	JobName string
}

func Handler(conf Config, g *echo.Group) error {
	s, err := NewServices(conf)
	if err != nil {
		return err
	}

	auth := serviceauth.Middleware(conf.Host, conf.ServiceAccounts, nil)

	// A subgroup keeps the auth off other routes registered later on the same group.
	g = g.Group("", auth)

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

		res, err := SetupCityItems(ctx, s, i, nil)
		if err != nil {
			log.Errorfc(ctx, "error: %v", err)
			return c.JSON(http.StatusBadGateway, err.Error())
		}

		return c.JSON(http.StatusOK, res)
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
