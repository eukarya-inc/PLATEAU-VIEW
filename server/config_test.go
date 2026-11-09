package main

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestConfig_Print(t *testing.T) {
	c := &Config{
		CMS_BaseURL: "https://cms.example.com",
		Secret:      "plain-secret",
		CMS_Token:   `tok"en\with-escapes`,
		FME_Token:   "日本語token",
	}

	s := c.Print()

	assert.Contains(t, s, "https://cms.example.com")
	assert.NotContains(t, s, "plain-secret")
	assert.NotContains(t, s, "tok")
	assert.NotContains(t, s, "日本語token")
	assert.Regexp(t, `CMS_Token:\s+"\*\*\*"`, s)
	// Print must not modify the original config
	assert.Equal(t, `tok"en\with-escapes`, c.CMS_Token)
}
