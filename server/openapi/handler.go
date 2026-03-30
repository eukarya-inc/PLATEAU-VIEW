package openapi

import (
	_ "embed"
	"encoding/json"

	"github.com/labstack/echo/v4"
	"gopkg.in/yaml.v3"
)

//go:embed openapi.yml
var y []byte
var j []byte

const docsHTML = `<!doctype html>
<html>
<head>
  <title>PLATEAU API Reference</title>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
</head>
<body>
  <script id="api-reference" data-url="/openapi.yml"></script>
  <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>`

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

		g.GET("/docs", func(c echo.Context) error {
			return c.HTML(200, docsHTML)
		})
	}

	if j != nil {
		g.GET("/openapi.json", func(c echo.Context) error {
			return c.Blob(200, "application/json", j)
		})
	}

	return nil
}
