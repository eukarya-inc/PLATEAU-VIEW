package tool

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"time"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintsetup"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"golang.org/x/oauth2"
	"google.golang.org/api/idtoken"
	"google.golang.org/api/impersonate"
)

func reconvertJGD2024(conf *Config, args []string) error {
	println("reconvert-jgd2024")

	var base, token, systemProject, projects, years, cities, features string
	inp := cmsintsetup.ReconvertJGD2024Input{}

	flags := flag.NewFlagSet("reconvert-jgd2024", flag.ExitOnError)
	flags.StringVar(&base, "base", conf.CMS_BaseURL, "CMS base URL")
	flags.StringVar(&token, "token", conf.CMS_Token, "CMS token")
	flags.StringVar(&systemProject, "system-project", conf.CMS_SystemProject, "CMS system project ID")
	flags.StringVar(&projects, "projects", "", "comma-separated <year>=<project ID>; all PLATEAU projects of the v3 schema in workspaces when empty")
	flags.StringVar(&years, "years", "", "comma-separated years of the projects found in workspaces")
	flags.StringVar(&cities, "cities", "", "comma-separated city codes")
	flags.StringVar(&features, "features", "", "comma-separated feature types (bldg, tran, ...)")
	flags.IntVar(&inp.Limit, "limit", 0, "maximum number of conversions to start (0: no limit)")
	flags.IntVar(&inp.Concurrency, "concurrency", 2, "number of conversions running at once")
	flags.IntVar(&inp.MaxFailures, "max-failures", 3, "stop starting conversions after this many failures (0: no limit)")
	var idToken, impersonateSA string
	flags.StringVar(&inp.ServerURL, "server", "https://api.plateauview.mlit.go.jp", "base URL of the PLATEAU VIEW server, which is also the audience of the id token")
	flags.StringVar(&impersonateSA, "impersonate-service-account", "", "service account whose ID tokens are made with your own credentials (needs roles/iam.serviceAccountOpenIdTokenCreator on it)")
	flags.StringVar(&idToken, "id-token", os.Getenv("RECONVERT_ID_TOKEN"), "Google ID token to use instead of one from Application Default Credentials (env RECONVERT_ID_TOKEN); it expires in an hour")
	flags.StringVar(&inp.StateFile, "state", "reconvert-jgd2024.jsonl", "file that records the results, used to resume")
	flags.DurationVar(&inp.PollInterval, "poll", 30*time.Second, "interval to check the status of a running conversion")
	flags.DurationVar(&inp.Timeout, "timeout", 6*time.Hour, "time to follow one conversion")
	flags.BoolVar(&inp.DryRun, "dryrun", false, "list the targets without converting them")
	if err := flags.Parse(args); err != nil {
		return err
	}

	if base == "" || token == "" || systemProject == "" {
		return errors.New("CMS base URL, CMS token and system project ID are required")
	}

	var err error
	if inp.Projects, err = parseProjects(projects); err != nil {
		return err
	}
	if inp.Years, err = parseInts(years); err != nil {
		return fmt.Errorf("invalid -years: %w", err)
	}
	inp.CityCodes = splitList(cities)
	inp.FeatureTypes = splitList(features)

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt)
	defer stop()

	if !inp.DryRun {
		// An ID token of a service account allowed by the server: on a Cloud Run Job, that of the
		// job's service account; locally, ADC that impersonates the service account.
		audience := strings.TrimSuffix(inp.ServerURL, "/")
		switch {
		case idToken != "":
			inp.Token = oauth2.StaticTokenSource(&oauth2.Token{AccessToken: idToken})
		case impersonateSA != "":
			// generateIdToken only, so the caller needs no right to act as the service account otherwise
			inp.Token, err = impersonate.IDTokenSource(ctx, impersonate.IDTokenConfig{
				Audience:        audience,
				TargetPrincipal: impersonateSA,
				IncludeEmail:    true,
			})
		default:
			inp.Token, err = idtoken.NewTokenSource(ctx, audience)
		}
		if err != nil {
			return fmt.Errorf("failed to get an id token source (use -impersonate-service-account locally): %w", err)
		}
	}

	services, err := cmsintsetup.NewServices(cmsintsetup.Config{
		CMSURL:           base,
		CMSToken:         token,
		CMSSystemProject: systemProject,
	})
	if err != nil {
		return fmt.Errorf("failed to initialize services: %w", err)
	}
	specs, ok := services.PCMS.(plateaucms.SpecStore)
	if !ok {
		return errors.New("plateau specs are not available")
	}

	sum, err := cmsintsetup.ReconvertJGD2024(ctx, services, specs, inp,
		func(t cmsintsetup.ReconvertTarget) {
			if t.Skip != "" {
				fmt.Printf("skip  %s: %s\n", t, t.Skip)
			} else {
				fmt.Printf("target %s\n", t)
			}
		},
		func(r cmsintsetup.ReconvertRecord) {
			fmt.Printf("%s %s %s\n", r.Result, r.Target, r.Message)
		},
	)
	if sum != nil {
		fmt.Println(sum)
	}
	if err != nil {
		return fmt.Errorf("failed to reconvert: %w", err)
	}
	return nil
}

func parseProjects(s string) ([]cmsintsetup.ReconvertProject, error) {
	var res []cmsintsetup.ReconvertProject
	for _, p := range splitList(s) {
		y, id, ok := strings.Cut(p, "=")
		year, err := strconv.Atoi(y)
		if !ok || err != nil || id == "" {
			return nil, fmt.Errorf("invalid -projects entry %q: want <year>=<project ID>", p)
		}
		res = append(res, cmsintsetup.ReconvertProject{ID: id, Year: year})
	}
	return res, nil
}

func parseInts(s string) ([]int, error) {
	var res []int
	for _, v := range splitList(s) {
		i, err := strconv.Atoi(v)
		if err != nil {
			return nil, err
		}
		res = append(res, i)
	}
	return res, nil
}

func splitList(s string) []string {
	var res []string
	for _, v := range strings.Split(s, ",") {
		if v = strings.TrimSpace(v); v != "" {
			res = append(res, v)
		}
	}
	return res
}
