package datacatalog

import (
	"context"
	"path"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/datacatalogmcp"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/labstack/echo/v4"
	"github.com/labstack/echo/v4/middleware"
)

func echov3(conf Config, g *echo.Group, pcms *plateaucms.CMS) (func(ctx context.Context) error, error) {
	h, err := NewReposHandler(conf, pcms)
	if err != nil {
		return nil, err
	}

	// PLATEAU API
	plateauapig := g.Group("")
	plateauapig.Use(
		middleware.CORS(),
		middleware.Gzip(),
		h.Middleware(),
	)

	// GraphQL playground
	plateauapig.GET("/graphql", gqlPlaygroundHandler(conf.PlaygroundEndpoint, false))
	plateauapig.GET("/:pid/graphql", gqlPlaygroundHandler(conf.PlaygroundEndpoint, false))
	plateauapig.GET("/admin/graphql", gqlPlaygroundHandler(conf.PlaygroundEndpoint, true))
	plateauapig.GET("/:pid/admin/graphql", gqlPlaygroundHandler(conf.PlaygroundEndpoint, true))

	// GraphQL API
	plateauapig.POST("/graphql", h.Handler(false))
	plateauapig.POST("/:pid/graphql", h.Handler(false))
	plateauapig.POST("/admin/graphql", h.Handler(true))
	plateauapig.POST("/:pid/admin/graphql", h.Handler(true))

	// CityGML files API
	plateauapig.GET("/citygml/:conditions", h.CityGMLFiles(false))
	plateauapig.GET("/:pid/citygml/:conditions", h.CityGMLFiles(false))
	plateauapig.GET("/admin/citygml/:conditions", h.CityGMLFiles(true))
	plateauapig.GET("/:pid/admin/citygml/:conditions", h.CityGMLFiles(true))

	// Simple PLATEAU dataset API
	plateauapig.GET("/plateau-datasets", h.SimplePlateauDatasetsAPI())

	// warning API
	plateauapig.GET("/:pid/warnings", h.WarningHandler)

	// cache update API
	g.POST("/update-cache", h.UpdateCacheHandler)

	// Data catalog MCP API
	mcpService := datacatalogmcp.NewService(h, conf.Host)
	mcpGroup := g.Group("/mcp")
	mcpGroup.Use(
		middleware.CORS(),
		h.Middleware(),
	)
	mcpService.RegisterRoutes(mcpGroup)

	// Return initialization function
	// If CacheURL is set, load from cache instead of CMS
	if conf.CacheURL != "" {
		return func(ctx context.Context) error {
			return h.InitFromCache(ctx, conf.CacheURL)
		}, nil
	}

	return func(ctx context.Context) error {
		return h.Init(ctx)
	}, nil
}

func gqlPlaygroundHandler(endpoint string, admin bool) echo.HandlerFunc {
	return func(c echo.Context) error {
		pid := c.Param(pidParamName)

		p := make([]string, 0, 4)
		p = append(p, endpoint)
		if pid != "" {
			p = append(p, pid)
		}
		if admin {
			p = append(p, "admin")
		}
		p = append(p, "graphql")

		endpoint := path.Join(p...)
		if isAlpha(c) {
			endpoint += "?alpha=true"
		}

		h := plateauapi.PlaygroundHandler(
			"PLATEAU GraphQL API Playground",
			endpoint,
		)
		h.ServeHTTP(c.Response(), c.Request())
		return nil
	}
}
