package main

import (
	"fmt"
	"os"

	"github.com/joho/godotenv"
	"github.com/kelseyhightower/envconfig"
	"github.com/reearth/reearthx/log"
)

const configPrefix = "REEARTH"

type Config struct {
	CMS_URL   string
	CMS_Token string
}

func NewConfig() (*Config, error) {
	if err := godotenv.Load(".env"); err != nil && !os.IsNotExist(err) {
		return nil, err
	} else if err == nil {
		log.Infof("config: .env loaded")
	}

	var c Config
	err := envconfig.Process(configPrefix, &c)

	return &c, err
}

// Print returns the config for logging, with the CMS token masked.
func (c *Config) Print() string {
	r := *c
	if r.CMS_Token != "" {
		r.CMS_Token = "***"
	}
	return fmt.Sprintf("%+v", r)
}
