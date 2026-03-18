package datacatalogv3

import (
	"context"
	"encoding/csv"
	"os"
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/joho/godotenv"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func initCMS(run bool) (*cms.CMS, *plateaucms.CMS, string) {
	if !run {
		return nil, nil, ""
	}

	_ = godotenv.Load("../../.env")
	cmsurl := os.Getenv("REEARTH_PLATEAUVIEW_CMS_BASEURL")
	cmstoken := os.Getenv("REEARTH_PLATEAUVIEW_CMS_TOKEN")
	prj := os.Getenv("REEARTH_PLATEAUVIEW_TEST_CMS_PROJECT")
	if cmsurl == "" || cmstoken == "" || prj == "" {
		return nil, nil, ""
	}

	c, err := cms.New(cmsurl, cmstoken)
	if err != nil {
		panic(err)
	}

	pcms, err := plateaucms.New(plateaucms.Config{
		CMSBaseURL:   cmsurl,
		CMSMainToken: cmstoken,
	})
	if err != nil {
		panic(err)
	}

	return c, pcms, prj
}

func TestExtractDataFromCMS(t *testing.T) {
	run := false

	c, pcms, prj := initCMS(run)
	if c == nil {
		t.Skip("skip")
	}

	ctx := context.Background()
	c2 := NewCMS(CMSOpts{
		CMS:     c,
		PCMS:    pcms,
		Year:    2023,
		Plateau: true,
		Project: prj,
		Cache:   false,
	})
	all, err := c2.GetAll(ctx, "")
	assert.NoError(t, err)

	t.Log("get all data done")

	// do something with all
	records := [][]string{}

	for _, city := range all.City {
		g := all.FindGspatialjpDataItemByCityID(city.ID)
		if g == nil {
			continue
		}

		if !g.HasIndex {
			continue
		}

		r := []string{city.ID, city.CityName, city.CityCode}
		records = append(records, r)
	}

	f, err := os.Create("cities.csv")
	assert.NoError(t, err)
	w := csv.NewWriter(f)
	_ = w.WriteAll(records)
	w.Flush()
	_ = f.Close()
}

func TestFeatureTypes(t *testing.T) {
	run := false

	c, pcms, prj := initCMS(run)
	if c == nil {
		t.Skip("skip")
	}

	cms := NewCMS(CMSOpts{
		CMS:     c,
		PCMS:    pcms,
		Year:    2023,
		Plateau: true,
		Project: prj,
	})

	ctx := context.Background()
	res, err := cms.GetFeatureTypes(ctx)
	require.NoError(t, err)

	res2, err := getFeatureTypes(ctx, pcms)
	require.NoError(t, err)

	for i := range res.Plateau {
		t.Run(res.Plateau[i].Code, func(t *testing.T) {
			ok := false
			for _, j := range res2.Plateau {
				if res.Plateau[i].Code == j.Code {
					assert.Equal(t, res.Plateau[i], j)
					ok = true
				}
			}
			assert.True(t, ok)
		})
	}
}
