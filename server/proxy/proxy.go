package proxy

import (
	"net/url"

	"github.com/labstack/echo/v4"
	"github.com/labstack/echo/v4/middleware"
)

const odptHost = "api.odpt.org"

type Config struct {
	// ODPTConsumerKey is the API key for https://api.odpt.org/ injected as
	// `acl:consumerKey` on every forwarded request. Empty disables the endpoint.
	ODPTConsumerKey string
}

// Echo registers GET /<path...> that proxies to https://api.odpt.org/<path...>
// with the configured consumerKey automatically appended to the query.
func Echo(g *echo.Group, conf Config) {
	target := &url.URL{Scheme: "https", Host: odptHost}
	proxyMW := middleware.ProxyWithConfig(middleware.ProxyConfig{
		Balancer: middleware.NewRoundRobinBalancer([]*middleware.ProxyTarget{{URL: target}}),
	})(func(c echo.Context) error { return nil })

	g.GET("/*", func(c echo.Context) error {
		req := c.Request()
		req.URL.Path = "/" + c.Param("*")
		q := req.URL.Query()
		q.Set("acl:consumerKey", conf.ODPTConsumerKey)
		req.URL.RawQuery = q.Encode()
		req.Host = odptHost
		return proxyMW(c)
	})
}
