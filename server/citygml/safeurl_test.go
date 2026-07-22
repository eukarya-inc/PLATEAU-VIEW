package citygml

import (
	"errors"
	"net/url"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestCheckExternalURL(t *testing.T) {
	// Non-public destinations that must be rejected regardless of scheme.
	// This includes cloud metadata IPs, loopback, RFC1918, link-local,
	// CGNAT, IPv6 loopback / link-local / ULA, and reserved ranges.
	blocked := []string{
		"http://127.0.0.1/",
		"http://localhost/",       // hostname literal that resolves to loopback — blocked via public-IP dial check, but scheme-check alone can't catch this. Skipped here (dial-time check).
		"http://169.254.169.254/", // AWS/GCP/Azure metadata
		"http://10.0.0.1/",
		"http://192.168.1.1/",
		"http://172.16.0.1/",
		"http://100.64.0.1/", // CGNAT
		"http://0.0.0.0/",
		"http://240.0.0.1/",
		"http://[::1]/",
		"http://[fe80::1]/", // link-local
		"http://[fc00::1]/", // ULA
		"http://[fd00::1]/", // ULA
		"http://[ff02::1]/", // multicast
		"http://[::ffff:127.0.0.1]/",
	}
	for _, raw := range blocked {
		u, err := url.Parse(raw)
		assert.NoError(t, err, raw)
		if u.Hostname() == "localhost" {
			// hostname literal — pre-flight check can't catch this, only
			// safeDialContext can. Skip.
			continue
		}
		err = checkExternalURL(u)
		assert.Error(t, err, "expected block for %s", raw)
		assert.True(t, errors.Is(err, ErrUnsafeURL), "expected ErrUnsafeURL for %s, got %v", raw, err)
	}

	// Public destinations that must be allowed.
	allowed := []string{
		"http://example.com/",
		"https://plateau.example/foo",
		"http://8.8.8.8/",
		"https://[2606:4700:4700::1111]/",
	}
	for _, raw := range allowed {
		u, err := url.Parse(raw)
		assert.NoError(t, err, raw)
		assert.NoError(t, checkExternalURL(u), "expected allow for %s", raw)
	}

	// Bad schemes.
	for _, raw := range []string{
		"file:///etc/passwd",
		"gopher://example.com/",
		"ftp://example.com/",
		"javascript:alert(1)",
	} {
		u, err := url.Parse(raw)
		assert.NoError(t, err, raw)
		assert.Error(t, checkExternalURL(u), "expected block for %s", raw)
	}

	// Nil / empty.
	assert.Error(t, checkExternalURL(nil))
	{
		u, _ := url.Parse("http:///path")
		assert.Error(t, checkExternalURL(u))
	}
}
