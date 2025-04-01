package openapi

import (
	_ "embed"
	"encoding/json"
	"net/http"

	"github.com/go-openapi/runtime/middleware"
	"github.com/labstack/echo/v4"
	"gopkg.in/yaml.v3"
)

//go:embed openapi.yml
var y []byte
var j []byte

func init() {
	var t any
	err := yaml.Unmarshal(y, &t)
	if err != nil {
		panic(err)
	}

	j, err = json.Marshal(t)
	if err != nil {
		panic(err)
	}
}

func Handler(g *echo.Group) error {
	if y != nil {
		g.GET("/openapi.yml", func(c echo.Context) error {
			return c.Blob(200, "application/x-yaml", y)
		})

		docs := echo.WrapMiddleware(func(h http.Handler) http.Handler {
			return middleware.SwaggerUI(middleware.SwaggerUIOpts{
				Path:    "/docs",
				SpecURL: "/openapi.yml",
			}, h)
		})(func(c echo.Context) error {
			return echo.ErrNotFound
		})

		g.GET("/docs/*", func(c echo.Context) error {
			return docs(c)
		})
	}

	if j != nil {
		g.GET("/openapi.json", func(c echo.Context) error {
			return c.Blob(200, "application/json", j)
		})
	}

	return nil
}
