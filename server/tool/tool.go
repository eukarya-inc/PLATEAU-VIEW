package tool

import (
	"errors"
	"fmt"
	"os"
)

type Config struct {
	CMS_BaseURL       string
	CMS_Token         string
	CMS_SystemProject string
}

func Main(conf *Config, args []string) {
	subommand := args[0]
	var err error

	args = args[1:]
	switch subommand {
	case "setup-city-items":
		err = setupCityItems(conf, args)
	case "copy-related":
		err = copyRelatedItems(conf, args)
	case "migrate-v1":
		err = migrateV1(conf, args)
	case "upload-assets":
		err = uploadAssets(conf, args)
	case "help":
		err = help(conf)
	default:
		err = errors.New("invalid subcommand")
	}

	if err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}

func help(*Config) error {
	fmt.Println(`Usage: plateauview <command> [arguments] [options] [flags]`)
	return nil
}
