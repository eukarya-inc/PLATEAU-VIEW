package geocoding

import (
	"github.com/99designs/gqlgen/graphql/handler"
	"github.com/99designs/gqlgen/graphql/playground"
	"github.com/labstack/echo/v4"
)

type HandlerConfig struct {
	GSIURL string
}

func Echo(g *echo.Group, conf *HandlerConfig) error {
	gsiURL := ""
	if conf != nil {
		gsiURL = conf.GSIURL
	}

	gsiClient := NewGSIClient(nil, gsiURL)
	resolver := NewResolver(gsiClient)

	srv := handler.NewDefaultServer(NewExecutableSchema(Config{
		Resolvers: resolver,
	}))

	g.POST("/graphql", echo.WrapHandler(srv))
	g.GET("/graphql", echo.WrapHandler(playground.Handler("Geocoding", "/geocoding/graphql")))

	return nil
}
