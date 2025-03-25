package tool

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"net/http"

	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintsetup"
	"github.com/k0kubun/pp/v3"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/samber/lo"
)

func copyRelatedItems(conf *Config, args []string) error {
	println("copy-related")

	var base, token string
	opts := cmsintsetup.CopyRelatedItemsOpts{}

	flags := flag.NewFlagSet("copy-related", flag.ExitOnError)
	flags.StringVar(&base, "base", conf.CMS_BaseURL, "CMS base URL")
	flags.StringVar(&token, "token", conf.CMS_Token, "CMS token")
	flags.StringVar(&opts.FromProject, "src", "", "src project ID")
	flags.StringVar(&opts.ToProject, "target", "", "target project ID")
	flags.IntVar(&opts.Offset, "offset", 0, "offset")
	flags.IntVar(&opts.Limit, "limit", 0, "limit")
	flags.BoolVar(&opts.DryRun, "dryrun", false, "dryrun")
	flags.BoolVar(&opts.Force, "force", false, "force")
	flags.StringVar(&opts.CacheBasePath, "cache", "", "cache base path")
	if err := flags.Parse(args); err != nil {
		return err
	}

	if base == "" || token == "" || opts.FromProject == "" || opts.ToProject == "" {
		if base == "" {
			fmt.Println("CMS base URL is required")
		}
		if token == "" {
			fmt.Println("CMS token is required")
		}
		if opts.FromProject == "" {
			fmt.Println("fromProject is required")
		}
		if opts.ToProject == "" {
			fmt.Println("toProject is required")
		}
		return errors.New("CMS base URL, CMS token, fromProject, and toProject are required")
	}

	pp.Printf("args: %v\n", opts)

	err := cmsintsetup.CopyRelatedDatasetItems(
		context.Background(),
		&cmsintsetup.Services{
			CMS:  lo.Must(cms.New(base, token)),
			HTTP: http.DefaultClient,
		},
		opts,
	)

	if err != nil {
		return fmt.Errorf("failed to copy related items: %w", err)
	}

	return nil
}
