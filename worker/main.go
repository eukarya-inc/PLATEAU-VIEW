package main

import (
	"context"
	"flag"
	"os"
	"runtime"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/worker/citygmlpacker"
	"github.com/eukarya-inc/PLATEAU-VIEW/worker/extractmaxlod"
	"github.com/eukarya-inc/PLATEAU-VIEW/worker/lodstat"
	"github.com/eukarya-inc/PLATEAU-VIEW/worker/preparegspatialjp"
	"github.com/k0kubun/pp/v3"
	"github.com/samber/lo"
)

func init() {
	pp.ColoringEnabled = false
}

func main() {
	config := lo.Must(NewConfig())

	switch os.Args[1] {
	case "prepare-gspatialjp":
		prepareGspatialjp(config)
	case "extract-maxlod":
		extractMaxLOD(config)
	case "citygml-packer":
		cityGMLPacker(config)
	case "lodstat":
		lodStat(config)
	}
}

func prepareGspatialjp(conf *Config) {
	config := preparegspatialjp.Config{
		CMSURL:   conf.CMS_URL,
		CMSToken: conf.CMS_Token,
	}

	ft := ""
	flag := flag.NewFlagSet("prepare-gspatialjp", flag.ExitOnError)
	flag.StringVar(&config.ProjectID, "project", "", "CMS project ID")
	flag.StringVar(&config.CityItemID, "city", "", "CMS city item ID")
	flag.BoolVar(&config.WetRun, "wetrun", false, "wet run")
	flag.BoolVar(&config.Clean, "clean", false, "clean")
	flag.BoolVar(&config.SkipCityGML, "skip-citygml", false, "skip citygml")
	flag.BoolVar(&config.SkipPlateau, "skip-plateau", false, "skip plateau")
	flag.BoolVar(&config.SkipMaxLOD, "skip-maxlod", false, "skip maxlod")
	flag.BoolVar(&config.SkipRelated, "skip-related", false, "skip related")
	flag.StringVar(&ft, "feature-types", "", "feature types")

	if err := flag.Parse(os.Args[2:]); err != nil {
		panic(err)
	}

	config.FeatureTypes = strings.Split(ft, ",")
	if err := preparegspatialjp.Command(&config); err != nil {
		panic(err)
	}
}

func extractMaxLOD(conf *Config) {
	config := extractmaxlod.Config{
		CMSURL:   conf.CMS_URL,
		CMSToken: conf.CMS_Token,
	}

	itemID := ""
	featureTypes := ""

	flag := flag.NewFlagSet("extract-maxlod", flag.ExitOnError)
	flag.StringVar(&config.ProjectID, "project", "", "CMS project ID")
	flag.StringVar(&itemID, "city", "", "CMS item ID")
	flag.StringVar(&featureTypes, "ftypes", "", "feature types")
	flag.BoolVar(&config.WetRun, "wetrun", false, "wet run")
	flag.BoolVar(&config.Clean, "clean", false, "clean")
	flag.BoolVar(&config.Overwrite, "overwrite", false, "overwrite")

	if err := flag.Parse(os.Args[2:]); err != nil {
		panic(err)
	}

	if itemID != "" {
		config.CityItemID = strings.Split(itemID, ",")
	}

	if featureTypes != "" {
		config.FeatureTypes = strings.Split(featureTypes, ",")
	}

	if err := extractmaxlod.Run(config); err != nil {
		panic(err)
	}
}

func cityGMLPacker(*Config) {
	var config citygmlpacker.Config
	flag := flag.NewFlagSet("citygml-packer", flag.ExitOnError)
	flag.StringVar(&config.Dest, "dest", "", "destination url (gs://...)")
	flag.StringVar(&config.Source, "source", "", "source url (gs://...)")
	flag.StringVar(&config.Domain, "domain", "", "allowed domain")
	flag.DurationVar(&config.Timeout, "timeout", 0, "timeout")
	if err := flag.Parse(os.Args[2:]); err != nil {
		panic(err)
	}
	config.URLs = lo.FlatMap(flag.Args(), func(s string, _ int) []string {
		return strings.Split(s, ",")
	})

	if err := citygmlpacker.Run(config); err != nil {
		panic(err)
	}
}

func lodStat(c *Config) {
	var config lodstat.Config
	config.CMSURL = c.CMS_URL
	config.CMSToken = c.CMS_Token

	flag := flag.NewFlagSet("lodstat", flag.ExitOnError)
	flag.StringVar(&config.SrcURL, "src", "", "src")
	flag.StringVar(&config.ProjectID, "project", "", "project")
	flag.StringVar(&config.ItemID, "item", "", "item")
	flag.StringVar(&config.Feature, "feature", "", "feature")
	flag.IntVar(&config.Parallelism, "p", runtime.GOMAXPROCS(0), "parallelism")
	if err := flag.Parse(os.Args[2:]); err != nil {
		panic(err)
	}
	if err := lodstat.Run(context.Background(), config); err != nil {
		panic(err)
	}
}
