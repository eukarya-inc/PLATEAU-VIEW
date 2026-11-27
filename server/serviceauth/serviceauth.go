// Package serviceauth authenticates the operation routes of the server with Google ID tokens
// of service accounts, instead of shared secrets.
package serviceauth

import (
	"context"
	"net/http"
	"slices"
	"strings"

	"github.com/labstack/echo/v4"
	"github.com/reearth/reearthx/log"
	"google.golang.org/api/idtoken"
)

// IDTokenValidator validates a Google ID token for the audience and returns its claims.
type IDTokenValidator func(ctx context.Context, token, audience string) (map[string]any, error)

// ValidateGoogleIDToken checks the signature, expiry and audience of a Google ID token.
func ValidateGoogleIDToken(ctx context.Context, token, audience string) (map[string]any, error) {
	p, err := idtoken.Validate(ctx, token, audience)
	if err != nil {
		return nil, err
	}
	return p.Claims, nil
}

// Middleware lets through only the requests that carry a Google ID token of one of the
// service accounts, issued for the audience (the server's URL). Operation tools run as those
// service accounts, on Cloud Run Jobs or locally by impersonating them, so no shared secret is
// needed. Every request is rejected when no service account or no audience is configured.
func Middleware(audience string, serviceAccounts []string, validate IDTokenValidator) echo.MiddlewareFunc {
	if validate == nil {
		validate = ValidateGoogleIDToken
	}
	audience = strings.TrimSuffix(audience, "/")
	accounts := make([]string, 0, len(serviceAccounts))
	for _, a := range serviceAccounts {
		if a = strings.TrimSpace(a); a != "" {
			accounts = append(accounts, a)
		}
	}
	return func(next echo.HandlerFunc) echo.HandlerFunc {
		return func(c echo.Context) error {
			if audience == "" || len(accounts) == 0 {
				return c.JSON(http.StatusUnauthorized, "no service account is allowed")
			}
			token, ok := strings.CutPrefix(c.Request().Header.Get("Authorization"), "Bearer ")
			if !ok || token == "" {
				return c.JSON(http.StatusUnauthorized, "missing id token")
			}
			claims, err := validate(c.Request().Context(), token, audience)
			if err != nil {
				log.Debugfc(c.Request().Context(), "invalid id token: %v", err)
				return c.JSON(http.StatusUnauthorized, "invalid id token")
			}
			if iss, _ := claims["iss"].(string); iss != "https://accounts.google.com" && iss != "accounts.google.com" {
				return c.JSON(http.StatusUnauthorized, "invalid id token issuer")
			}
			email, _ := claims["email"].(string)
			verified, _ := claims["email_verified"].(bool)
			if !verified || !slices.Contains(accounts, email) {
				return c.JSON(http.StatusForbidden, "service account is not allowed")
			}
			return next(c)
		}
	}
}
