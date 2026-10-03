package file

import (
	"context"
	"errors"
	"fmt"
	"net"
	"net/http"
	"net/url"
	"strings"
	"syscall"
	"time"
)

// ErrUnsafeURL is returned when a caller-supplied URL (e.g. `POST /api/.../
// assets {"url":...}`) points at something the asset importer refuses to
// fetch: bad scheme, or a resolved address in a private / loopback / cloud-
// metadata range. Surfaces as an internal error to the caller.
var ErrUnsafeURL = errors.New("unsafe url")

// checkExternalURL runs a pre-flight SSRF check on the raw URL. It rejects
// non-http/https schemes and IP literals in a non-public range. Actual DNS
// resolution is re-checked in safeDialContext below so a rebinding attacker
// who returns a public A record here and a private one at connect time is
// still stopped.
func checkExternalURL(u *url.URL) error {
	if u == nil {
		return fmt.Errorf("%w: nil url", ErrUnsafeURL)
	}
	switch strings.ToLower(u.Scheme) {
	case "http", "https":
	default:
		return fmt.Errorf("%w: scheme %q not allowed", ErrUnsafeURL, u.Scheme)
	}
	host := u.Hostname()
	if host == "" {
		return fmt.Errorf("%w: empty host", ErrUnsafeURL)
	}
	if ip := net.ParseIP(host); ip != nil && !isPublicIP(ip) {
		return fmt.Errorf("%w: address %s is not public", ErrUnsafeURL, ip)
	}
	return nil
}

// isPublicIP mirrors the same-named helper in the sibling PLATEAU server
// citygml package: return true only for addresses safe to open an outbound
// connection to from a workspace-tenant-facing service.
func isPublicIP(ip net.IP) bool {
	if ip == nil {
		return false
	}
	if ip.IsLoopback() || ip.IsPrivate() || ip.IsLinkLocalUnicast() ||
		ip.IsLinkLocalMulticast() || ip.IsMulticast() || ip.IsUnspecified() ||
		ip.IsInterfaceLocalMulticast() {
		return false
	}
	if v4 := ip.To4(); v4 != nil {
		// 169.254.169.254 (cloud metadata), 100.64.0.0/10 (CGNAT),
		// 0.0.0.0/8, 240.0.0.0/4 (reserved).
		if v4[0] == 0 || v4[0] >= 240 {
			return false
		}
		if v4[0] == 169 && v4[1] == 254 {
			return false
		}
		if v4[0] == 100 && v4[1] >= 64 && v4[1] <= 127 {
			return false
		}
		return true
	}
	// IPv6 unique-local (fc00::/7).
	if len(ip) == net.IPv6len && (ip[0]&0xfe) == 0xfc {
		return false
	}
	return true
}

// safeDialContext refuses to connect to non-public addresses even after DNS
// resolution — the defense-in-depth check that closes the DNS-rebinding gap
// left by a purely pre-flight URL check.
//
// Dials each validated public IP in resolver order, returning the first
// success. This keeps dual-stack (A + AAAA) hosts working even when one
// family is unreachable — e.g. a container network with no IPv6 route
// would otherwise fail even though the A record resolved fine.
func safeDialContext(ctx context.Context, network, address string) (net.Conn, error) {
	host, port, err := net.SplitHostPort(address)
	if err != nil {
		return nil, err
	}
	ips, err := (&net.Resolver{}).LookupIPAddr(ctx, host)
	if err != nil {
		return nil, err
	}
	if len(ips) == 0 {
		return nil, fmt.Errorf("%w: no addresses resolved for %s", ErrUnsafeURL, host)
	}
	// Validate all resolved IPs up front. Any non-public one aborts the
	// dial: partial-public / partial-private records could otherwise let
	// an attacker exfiltrate to an internal address by dominating the
	// resolver order.
	for _, ipa := range ips {
		if !isPublicIP(ipa.IP) {
			return nil, fmt.Errorf("%w: resolved address %s not public", ErrUnsafeURL, ipa.IP)
		}
	}
	dialer := &net.Dialer{
		Timeout:   10 * time.Second,
		KeepAlive: 30 * time.Second,
		Control: func(network, address string, c syscall.RawConn) error {
			host, _, err := net.SplitHostPort(address)
			if err != nil {
				return err
			}
			if ip := net.ParseIP(host); ip != nil && !isPublicIP(ip) {
				return fmt.Errorf("%w: dial to non-public address %s blocked", ErrUnsafeURL, ip)
			}
			return nil
		},
	}
	// Try each validated IP; return on first success. `lastErr` is the
	// last dial error so callers see the actual TCP-level failure when
	// none of the IPs are reachable.
	var lastErr error
	for _, ipa := range ips {
		conn, err := dialer.DialContext(ctx, network, net.JoinHostPort(ipa.IP.String(), port))
		if err == nil {
			return conn, nil
		}
		lastErr = err
		// Respect context cancellation so a caller-side timeout doesn't
		// wait through every remaining IP.
		if ctx.Err() != nil {
			break
		}
	}
	return nil, lastErr
}

// safeHTTPClient is the SSRF-safe client used for caller-supplied URLs in
// `file.FromURL` (asset import-from-URL via the integration API / GraphQL
// createAsset). Rejects non-public destinations at dial time, limits
// redirects, and re-validates each redirect target.
//
// `Proxy: nil` is explicit: with `http.ProxyFromEnvironment` an operator-
// set `HTTP(S)_PROXY` would let `DialContext` dial the proxy — a public
// address — while the CONNECT tunnel terminated at an internal target,
// defeating the public-IP check.
var safeHTTPClient = &http.Client{
	Timeout: 60 * time.Second,
	Transport: &http.Transport{
		Proxy:                 nil,
		DialContext:           safeDialContext,
		ForceAttemptHTTP2:     true,
		MaxIdleConns:          100,
		IdleConnTimeout:       90 * time.Second,
		TLSHandshakeTimeout:   10 * time.Second,
		ExpectContinueTimeout: 1 * time.Second,
	},
	CheckRedirect: func(req *http.Request, via []*http.Request) error {
		if len(via) >= 10 {
			return errors.New("stopped after 10 redirects")
		}
		return checkExternalURL(req.URL)
	},
}
