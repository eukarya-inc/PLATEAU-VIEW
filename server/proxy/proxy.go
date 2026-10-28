package proxy

import (
	"net/http"
	"net/url"

	"github.com/labstack/echo/v4"
	"github.com/labstack/echo/v4/middleware"
)

const odptHost = "api.odpt.org"

// allowedPaths lists the api.odpt.org paths that may be forwarded. They are the
// GTFS-Realtime feeds referenced by the usecase datasets in the data catalog.
// Add a path here when a new feed is registered.
var allowedPaths = map[string]struct{}{
	"/api/v4/gtfs/realtime/SeibuBus_vehicle":             {},
	"/api/v4/gtfs/realtime/YokohamaMunicipalBus_vehicle": {},
	"/api/v4/gtfs/realtime/ToeiBus":                      {},
}

type Config struct {
	// ODPTConsumerKey is the API key for https://api.odpt.org/ injected as
	// `acl:consumerKey` on every forwarded request. Empty disables the endpoint.
	ODPTConsumerKey string
}

// Echo registers GET /<path...> that proxies to https://api.odpt.org/<path...>
// with the configured consumerKey automatically appended to the query.
// Only the paths in allowedPaths are forwarded.
func Echo(g *echo.Group, conf Config) {
	echoWithTarget(g, conf, &url.URL{Scheme: "https", Host: odptHost})
}

func echoWithTarget(g *echo.Group, conf Config, target *url.URL) {
	proxyMW := middleware.ProxyWithConfig(middleware.ProxyConfig{
		Balancer: middleware.NewRoundRobinBalancer([]*middleware.ProxyTarget{{URL: target}}),
	})(func(c echo.Context) error { return nil })

	g.GET("/*", func(c echo.Context) error {
		p := "/" + c.Param("*")
		if _, ok := allowedPaths[p]; !ok {
			return c.JSON(http.StatusNotFound, "not found")
		}

		req := c.Request()
		req.URL.Path = p
		req.URL.RawPath = ""
		q := req.URL.Query()
		q.Set("acl:consumerKey", conf.ODPTConsumerKey)
		req.URL.RawQuery = q.Encode()
		req.Host = target.Host
		return proxyMW(c)
	})
}
