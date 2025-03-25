package extractmaxlod

import (
	"archive/zip"
	"context"
	"fmt"
	"io/fs"
	"math/rand"
	"os"
	"path"
	"path/filepath"
	"strings"
	"time"

	"slices"

	"github.com/eukarya-inc/reearth-plateauview/worker/workerutil"
	"github.com/k0kubun/pp/v3"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/log"
)

const tmpDirBase = "plateau-api-worker-tmp"
const model = "plateau-city"

func Run(conf Config) error {
	ctx := context.Background()
	log.Infofc(ctx, "extractmaxlod conf: %s", pp.Sprint(conf))

	// init cms
	cms, err := conf.CMS()
	if err != nil {
		return fmt.Errorf("failed to initialize CMS client: %w", err)
	}

	// init tmp dir
	tmpDirName := fmt.Sprintf("%s-%d", time.Now().Format("20060102-150405"), rand.Intn(1000))
	tmpDir := filepath.Join(tmpDirBase, tmpDirName)
	_ = os.MkdirAll(tmpDir, 0755)
	log.Infofc(ctx, "tmp dir: %s", tmpDir)

	// fetch cities
	log.Infofc(ctx, "fetching cities")
	cities, err := fetchCities(ctx, cms, conf)
	if err != nil {
		return fmt.Errorf("failed to fetch cities: %w", err)
	}

	failed := []string{}

	log.Infofc(ctx, "fetched cities: %d", len(cities))
	for i, city := range cities {
		log.Infofc(ctx, "processing city: %s (%d/%d)", city.Name, i+1, len(cities))
		if err := processCity(ctx, conf, cms, city, tmpDir); err != nil {
			log.Errorfc(ctx, "failed to process city: %s", city.Name)
			failed = append(failed, fmt.Sprintf("%s %s (%s)", city.Name, city.Code, city.ID))
		}
	}

	// clean
	if conf.Clean {
		log.Infofc(ctx, "cleaning tmp dir")
		_ = os.RemoveAll(tmpDir)
	}

	if len(failed) > 0 {
		return fmt.Errorf("failed to process cities: %v", failed)
	}

	return nil
}

func processCity(ctx context.Context, conf Config, cms *cms.CMS, city *City, tmpDir string) error {
	if len(city.References) == 0 {
		log.Infofc(ctx, "city is skipped: references is nil")
		return nil
	}

	failed := []string{}
	for _, ft := range featureTypes {
		if len(conf.FeatureTypes) > 0 {
			found := slices.Contains(conf.FeatureTypes, ft)
			if !found {
				continue
			}
		}

		ref := city.References[ft]
		if ref == "" {
			continue
		}

		log.Infofc(ctx, "processing feature type: %s", ft)
		if err := processFeatureType(ctx, conf, cms, ft, ref, tmpDir); err != nil {
			log.Errorfc(ctx, "failed to process feature type: %v", err)
			failed = append(failed, ft)
		}
	}

	if len(failed) > 0 {
		return fmt.Errorf("failed to process city: %v", failed)
	}

	return nil
}

func processFeatureType(ctx context.Context, conf Config, c *cms.CMS, ft, ref, tmpDir string) (err error) {
	defer func() {
		if err == nil || !conf.WetRun {
			return
		}

		if _, err := c.UpdateItem(ctx, ref, nil, []*cms.Field{
			{
				Key:   "maxlod_status",
				Type:  "tag",
				Value: "エラー",
			},
		}); err != nil {
			log.Errorfc(ctx, "failed to update feature status: %v", err)
		}

		_ = c.CommentToItem(ctx, ref, fmt.Sprintf("最大LOD抽出でエラーが発生しました。%s", err))
	}()

	maxlod := FixedMaxLOD[ft]
	if maxlod == "" {
		log.Infofc(ctx, "%s is skipped: maxlod is not fixed", ft)
		return nil
	}

	// fetch feature
	log.Infofc(ctx, "fetching feature: %s", ref)
	item, err := c.GetItem(ctx, ref, true)
	if err != nil {
		return fmt.Errorf("failed to fetch feature: %w", err)
	}

	feature := toFeature(item)
	if feature.CityGML == "" {
		log.Infofc(ctx, "feature is skipped: citygml is empty")
		return nil
	}

	if feature.MaxLOD != "" && !conf.Overwrite {
		log.Infofc(ctx, "feature is skipped: maxlod is already set")
		return nil
	}

	citygmlName := strings.TrimSuffix(path.Base(feature.CityGML), path.Ext(feature.CityGML))
	log.Infofc(ctx, "citygml name: %s", citygmlName)

	// download citygml
	log.Infofc(ctx, "downloading citygml: %s", feature.CityGML)
	err = workerutil.DownloadAndConsumeZip(ctx, feature.CityGML, tmpDir, func(zr *zip.Reader, fi fs.FileInfo) error {
		res := extractMaxLOD(zr, ft, maxlod)
		maxlodCSV := maxlodCSV(res)
		maxlodName := citygmlName + "_maxlod.csv"
		log.Infofc(ctx, "maxlod csv: %s\n%s", maxlodName, maxlodCSV)

		if conf.WetRun {
			// upload maxlod CSV
			log.Infofc(ctx, "uploading asset: %s", maxlodName)
			assetID, err := c.UploadAssetDirectly(ctx, conf.ProjectID, maxlodName, strings.NewReader(maxlodCSV))
			if err != nil {
				return fmt.Errorf("failed to upload asset: %w", err)
			}

			// update feature
			log.Infofc(ctx, "uploaded asset: %s", assetID)
			if _, err := c.UpdateItem(ctx, ref, []*cms.Field{
				{
					Key:   "maxlod",
					Value: assetID,
				},
			}, nil); err != nil {
				return fmt.Errorf("failed to update feature: %w", err)
			}

			log.Infofc(ctx, "updated feature: %s", ref)
		} else {
			log.Infofc(ctx, "upload was skipped")
		}

		return nil
	})

	if err != nil {
		return err
	}

	if conf.WetRun {
		if _, err := c.UpdateItem(ctx, ref, nil, []*cms.Field{
			{
				Key:   "maxlod_status",
				Type:  "tag",
				Value: "成功",
			},
		}); err != nil {
			log.Errorfc(ctx, "failed to update feature status: %v", err)
		}

		_ = c.CommentToItem(ctx, ref, "最大LOD抽出が完了しました。")
	}

	return nil
}
