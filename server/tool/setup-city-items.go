package tool

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"net/http"
	"os"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintsetup"
	"github.com/k0kubun/pp/v3"
)

func setupCityItems(conf *Config, args []string) error {
	println("setup-city-items")

	var base, token, file, systemProject string
	inp := cmsintsetup.SetupCityItemsInput{}

	flags := flag.NewFlagSet("setup-city-items", flag.ExitOnError)
	flags.StringVar(&base, "base", conf.CMS_BaseURL, "CMS base URL")
	flags.StringVar(&token, "token", conf.CMS_Token, "CMS token")
	flags.StringVar(&systemProject, "system-project", conf.CMS_SystemProject, "CMS system project ID")
	flags.StringVar(&inp.ProjectID, "project", "", "project ID")
	flags.StringVar(&file, "file", "", "file path")
	flags.BoolVar(&inp.Force, "force", false, "force")
	flags.IntVar(&inp.Offset, "offset", 0, "offset")
	flags.IntVar(&inp.Limit, "limit", 0, "limit")
	flags.BoolVar(&inp.DryRun, "dryrun", false, "dryrun")
	if err := flags.Parse(args); err != nil {
		return err
	}

	if base == "" || token == "" || systemProject == "" || inp.ProjectID == "" || file == "" {
		if base == "" {
			fmt.Println("CMS base URL is required")
		}
		if token == "" {
			fmt.Println("CMS token is required")
		}
		if systemProject == "" {
			fmt.Println("CMS system project ID is required")
		}
		if inp.ProjectID == "" {
			fmt.Println("project is required")
		}
		if file == "" {
			fmt.Println("file is required")
		}
		return errors.New("CMS base URL, CMS token, system project ID, project, and file are required")
	}

	_, _ = pp.Printf("args: %v\n", inp)

	f, err := os.Open(file)
	if err != nil {
		return fmt.Errorf("failed to open file: %w", err)
	}

	defer func() { _ = f.Close() }()

	inp.DataBody = f

	// Initialize services with NewServices to properly set up PCMS
	services, err := cmsintsetup.NewServices(cmsintsetup.Config{
		CMSURL:           base,
		CMSToken:         token,
		CMSSystemProject: systemProject,
	})
	if err != nil {
		return fmt.Errorf("failed to initialize services: %w", err)
	}
	services.HTTP = http.DefaultClient

	err = cmsintsetup.SetupCityItems(
		context.Background(),
		services,
		inp,
		func(i, l int, item cmsintsetup.SetupCSVItem) {
			fmt.Printf("processing %d/%d %s\n", i, l, item.Name)
		},
	)

	if err != nil {
		return fmt.Errorf("failed to setup city items: %w", err)
	}

	return nil
}
