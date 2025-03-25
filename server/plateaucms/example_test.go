package plateaucms

import (
	"context"
	"fmt"
	"os"

	"github.com/joho/godotenv"
)

const enableExample = false

func ExampleCMS_AllMetadata() {
	cms := initCMS()
	if cms == nil {
		return
	}

	ctx := context.Background()
	res, err := cms.AllMetadata(ctx, false)
	if err != nil {
		panic(err)
	}

	for _, r := range res {
		fmt.Printf("%#v\n", r)
	}

	// Output:
}

func ExampleCMS_PlateauFeatureTypes() {
	cms := initCMS()
	if cms == nil {
		return
	}

	ctx := context.Background()
	res, err := cms.PlateauFeatureTypes(ctx)
	if err != nil {
		panic(err)
	}

	for _, r := range res {
		fmt.Printf("%#v\n", r)
	}

	// Output:
}

func initCMS() *CMS {
	if !enableExample {
		return nil
	}

	_ = godotenv.Load("../.env")
	cmsBaseURL := os.Getenv("REEARTH_PLATEAUVIEW_CMS_BASEURL")
	cmsMainToken := os.Getenv("REEARTH_PLATEAUVIEW_CMS_TOKEN")
	if cmsBaseURL == "" || cmsMainToken == "" {
		return nil
	}

	c, _ := New(Config{
		CMSBaseURL:   cmsBaseURL,
		CMSMainToken: cmsMainToken,
	})
	return c
}
