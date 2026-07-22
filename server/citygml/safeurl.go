package citygml

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

// ErrUnsafeURL is returned when a caller-supplied URL points at something we
// refuse to fetch from the CityGML endpoints (bad scheme, or a resolved
// address that lives in a private / loopback / link-local range).
var ErrUnsafeURL = errors.New("unsafe url")

// checkExternalURL performs a static, pre-flight SSRF check against a
// caller-supplied URL for the CityGML fetchers. It rejects anything that is
// not http/https, and — when the host is an IP literal — anything in a
// non-public range (loopback / private / link-local / multicast / unspecified /
// unique-local IPv6 / IPv6 link-local). Actual DNS resolution is re-checked in
// safeDialContext below so a rebinding attacker who returned a public A record
// at check time and a private one at connect time is still stopped.
//
// This intentionally does not consult the operator's `Domain` allowlist —
// domain is a positive allow, this is a negative block, and both apply.
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

// isPublicIP returns true if the given address is safe to open an outbound
// connection to from a server that must not reach internal services (e.g.
// cloud metadata, RFC1918 hosts, loopback).
func isPublicIP(ip net.IP) bool {
	if ip == nil {
		return false
	}
	if ip.IsLoopback() || ip.IsPrivate() || ip.IsLinkLocalUnicast() ||
		ip.IsLinkLocalMulticast() || ip.IsMulticast() || ip.IsUnspecified() ||
		ip.IsInterfaceLocalMulticast() {
		return false
	}
	// IPv4-mapped IPv6: unwrap before further checks.
	if v4 := ip.To4(); v4 != nil {
		// 169.254.169.254 (AWS/GCP/Azure metadata), 100.64.0.0/10 (CGNAT),
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
	// IPv6 unique-local (fc00::/7). net.IP.IsPrivate covers this on Go 1.17+ —
	// keep the explicit check for defense in depth in case that changes.
	if len(ip) == net.IPv6len && (ip[0]&0xfe) == 0xfc {
		return false
	}
	return true
}

// safeDialContext refuses to connect to non-public addresses even after DNS
// resolution. This is the defense-in-depth check that closes the DNS-rebinding
// gap left by a purely pre-flight URL check.
func safeDialContext(ctx context.Context, network, address string) (net.Conn, error) {
	host, port, err := net.SplitHostPort(address)
	if err != nil {
		return nil, err
	}
	ips, err := (&net.Resolver{}).LookupIPAddr(ctx, host)
	if err != nil {
		return nil, err
	}
	for _, ipa := range ips {
		if !isPublicIP(ipa.IP) {
			return nil, fmt.Errorf("%w: resolved address %s not public", ErrUnsafeURL, ipa.IP)
		}
	}
	// Use the first resolved IP directly so the actual TCP connect target is
	// exactly what we validated (net.Dial re-resolves the name otherwise).
	dialer := &net.Dialer{
		Timeout:   10 * time.Second,
		KeepAlive: 30 * time.Second,
		Control: func(network, address string, c syscall.RawConn) error {
			// Belt-and-braces: also verify the address we're about to connect
			// to. `Control` receives the resolved sockaddr as a string.
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
	return dialer.DialContext(ctx, network, net.JoinHostPort(ips[0].IP.String(), port))
}

// safeHTTPClient is an SSRF-safe HTTP client for caller-supplied URLs. It uses
// safeDialContext to block connections to non-public addresses and also
// re-validates the redirect target on every hop.
//
// `Proxy` is explicitly nil (rather than defaulting to
// `http.ProxyFromEnvironment`): with a proxy configured, `DialContext` would
// dial the proxy — a public address — and the CONNECT tunnel could still
// terminate at an internal destination, bypassing our public-IP check
// entirely. If a validating proxy setup is ever needed, wire in a `Proxy`
// func that also runs `checkExternalURL` on the tunneled target.
var safeHTTPClient = &http.Client{
	Timeout: 30 * time.Second,
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
