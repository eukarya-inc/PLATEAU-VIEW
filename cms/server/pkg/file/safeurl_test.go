package file

import (
	"context"
	"errors"
	"net/url"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestCheckExternalURL_BlocksNonPublic(t *testing.T) {
	// The pre-flight check only inspects the URL as given — hostname
	// literals like "localhost" are handled by safeDialContext at connect
	// time (a full integration test would need a controllable resolver;
	// TestFromURL_RejectsUnsafe covers the IP-literal path end-to-end).
	for _, raw := range []string{
		"http://127.0.0.1/x",
		"http://169.254.169.254/latest/meta-data/",
		"http://10.0.0.1/",
		"http://192.168.1.1/",
		"http://172.16.0.1/",
		"http://100.64.0.1/",
		"http://0.0.0.0/",
		"http://240.0.0.1/",
		"http://[::1]/",
		"http://[fe80::1]/",
		"http://[fc00::1]/",
		"http://[fd00::1]/",
		"http://[ff02::1]/",
		"http://[::ffff:127.0.0.1]/",
	} {
		u, err := url.Parse(raw)
		assert.NoError(t, err, raw)
		err = checkExternalURL(u)
		if assert.Error(t, err, "expected block for %s", raw) {
			assert.True(t, errors.Is(err, ErrUnsafeURL), "expected ErrUnsafeURL for %s, got %v", raw, err)
		}
	}
}

func TestCheckExternalURL_AllowsPublic(t *testing.T) {
	for _, raw := range []string{
		"http://example.com/",
		"https://cms.com/xyz/test.txt",
		"http://8.8.8.8/",
		"https://[2606:4700:4700::1111]/",
	} {
		u, err := url.Parse(raw)
		assert.NoError(t, err, raw)
		assert.NoError(t, checkExternalURL(u), "expected allow for %s", raw)
	}
}

func TestCheckExternalURL_BadSchemes(t *testing.T) {
	for _, raw := range []string{
		"file:///etc/passwd",
		"gopher://example.com/",
		"ftp://example.com/file",
	} {
		u, err := url.Parse(raw)
		assert.NoError(t, err, raw)
		assert.Error(t, checkExternalURL(u), "expected block for %s", raw)
	}
}

// TestFromURL_RejectsUnsafe covers the FromURL integration: a caller URL
// pointing at cloud metadata must be rejected before any HTTP round-trip
// happens. `rerror.ErrInternalBy` conceals the wrapped cause in .Error(),
// so this just asserts the call fails — TestCheckExternalURL_BlocksNonPublic
// covers the underlying classification.
func TestFromURL_RejectsUnsafe(t *testing.T) {
	got, err := FromURL(context.Background(), "http://169.254.169.254/latest/meta-data/iam/security-credentials/")
	assert.Error(t, err)
	assert.Nil(t, got)
}
