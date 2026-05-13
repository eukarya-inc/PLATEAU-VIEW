package proxy

import (
	"net/http"
	"net/url"
	"strings"

	"github.com/labstack/echo/v4"
	"github.com/labstack/echo/v4/middleware"
)

type Config struct {
	// AllowedHosts is the list of hostnames the proxy is allowed to forward to.
	// Comparisons are case-insensitive and require exact host match.
	// When empty, the proxy endpoint is not registered.
	AllowedHosts []string
}

func Echo(g *echo.Group, conf Config) {
	g.GET("/*", handler(normalizeHosts(conf.AllowedHosts)))
}

func handler(allowed map[string]struct{}) echo.HandlerFunc {
	return func(c echo.Context) error {
		targetPath := c.Param("*")

		// This shouldn't be done by us but It'll do for now: @pyshx
		if strings.HasPrefix(targetPath, "http:/") && len(targetPath) > 6 && targetPath[6] != '/' {
			targetPath = "http://" + strings.TrimPrefix(targetPath, "http:/")
		} else if strings.HasPrefix(targetPath, "https:/") && len(targetPath) > 7 && targetPath[7] != '/' {
			targetPath = "https://" + strings.TrimPrefix(targetPath, "https:/")
		}

		targetURL, err := url.Parse(targetPath)
		if err != nil || !targetURL.IsAbs() {
			return c.JSON(http.StatusBadRequest, map[string]string{
				"error": "Invalid target URL",
			})
		}

		if targetURL.Scheme != "https" {
			return c.JSON(http.StatusBadRequest, map[string]string{
				"error": "Only https scheme is allowed",
			})
		}

		if _, ok := allowed[strings.ToLower(targetURL.Hostname())]; !ok {
			return c.JSON(http.StatusForbidden, map[string]string{
				"error": "Target host is not allowed",
			})
		}

		proxyConfig := middleware.ProxyConfig{
			Balancer: middleware.NewRoundRobinBalancer([]*middleware.ProxyTarget{
				{URL: targetURL},
			}),
		}

		proxyMiddleware := middleware.ProxyWithConfig(proxyConfig)(func(c echo.Context) error {
			return nil
		})

		return proxyMiddleware(c)
	}
}

func normalizeHosts(hosts []string) map[string]struct{} {
	out := make(map[string]struct{}, len(hosts))
	for _, h := range hosts {
		h = strings.ToLower(strings.TrimSpace(h))
		if h == "" {
			continue
		}
		out[h] = struct{}{}
	}
	return out
}
