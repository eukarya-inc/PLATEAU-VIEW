package main

import (
	"flag"
	"os"
	"strings"

	"github.com/eukarya-inc/reearth-plateauview/worker/extractmaxlod"
	"github.com/eukarya-inc/reearth-plateauview/worker/preparegspatialjp"
	"github.com/samber/lo"
)

func main() {
	config := lo.Must(NewConfig())

	switch os.Args[1] {
	case "prepare-gspatialjp":
		prepareGspatialjp(config)
	case "extract-maxlod":
		extractMaxLOD(config)
	}
}

func prepareGspatialjp(conf *Config) {
	config := preparegspatialjp.Config{
		CMSURL:   conf.CMS_URL,
		CMSToken: conf.CMS_Token,
	}

	flag := flag.NewFlagSet("prepare-gspatialjp", flag.ExitOnError)
	flag.StringVar(&config.ProjectID, "project", "", "CMS project ID")
	flag.StringVar(&config.CityItemID, "city", "", "CMS city item ID")
	flag.BoolVar(&config.WetRun, "wetrun", false, "wet run")
	flag.BoolVar(&config.Clean, "clean", false, "clean")
	flag.BoolVar(&config.SkipCityGML, "skip-citygml", false, "skip citygml")
	flag.BoolVar(&config.SkipPlateau, "skip-plateau", false, "skip plateau")
	flag.BoolVar(&config.SkipMaxLOD, "skip-maxlod", false, "skip maxlod")
	flag.BoolVar(&config.SkipRelated, "skip-related", false, "skip related")

	if err := flag.Parse(os.Args[2:]); err != nil {
		panic(err)
	}

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
