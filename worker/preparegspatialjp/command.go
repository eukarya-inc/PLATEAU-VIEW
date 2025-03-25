package preparegspatialjp

import (
	"context"
	"fmt"
	"math/rand"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/k0kubun/pp/v3"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/log"
)

const tmpDirBase = "plateau-api-worker-tmp"

type Config struct {
	CMSURL              string
	CMSToken            string
	ProjectID           string
	CityItemID          string
	SkipCityGML         bool
	SkipPlateau         bool
	SkipMaxLOD          bool
	SkipIndex           bool
	SkipRelated         bool
	ValidateMaxLOD      bool
	WetRun              bool
	Clean               bool
	SkipImcompleteItems bool
	IgnoreStatus        bool
	FeatureTypes        []string
}

type MergeContext struct {
	TmpDir             string
	CityItem           *CityItem
	AllFeatureItems    map[string]FeatureItem
	GspatialjpDataItem *GspatialjpDataItem
	WetRun             bool
	FeatureTypes       []string
}

func (m MergeContext) FileName(ty, suffix string) string {
	return m.CityItem.FileName(ty, suffix)
}

func CommandSingle(conf *Config) (err error) {
	ctx := context.Background()
	log.Infofc(ctx, "preparegeospatialjp conf: %s", pp.Sprint(conf))

	if conf == nil || conf.SkipCityGML && conf.SkipPlateau && conf.SkipMaxLOD && conf.SkipRelated && conf.SkipIndex && !conf.ValidateMaxLOD {
		return fmt.Errorf("no command to run")
	}

	if len(conf.FeatureTypes) == 0 {
		return fmt.Errorf("feature types is required")
	}

	cms, err := cms.New(conf.CMSURL, conf.CMSToken)
	if err != nil {
		return fmt.Errorf("failed to initialize CMS client: %w", err)
	}

	// get items fron CMS
	log.Infofc(ctx, "getting item from CMS...")

	cityItemRaw, err := cms.GetItem(ctx, conf.CityItemID, true)
	if err != nil {
		return fmt.Errorf("failed to get city item: %w", err)
	}
	log.Infofc(ctx, "city item raw: %s", pp.Sprint(cityItemRaw))

	cityItem := CityItemFrom(cityItemRaw, conf.FeatureTypes)
	log.Infofc(ctx, "city item: %s", pp.Sprint(cityItem))

	if cityItem == nil || cityItem.CityCode == "" || cityItem.CityName == "" || cityItem.CityNameEn == "" || cityItem.GeospatialjpData == "" {
		if conf.SkipImcompleteItems {
			log.Infofc(ctx, "skip because city item is incomplete")
			return nil
		}
		return fmt.Errorf("invalid city item: %s", conf.CityItemID)
	}

	indexItemRaw, err := cms.GetItem(ctx, cityItem.GeospatialjpIndex, false)
	if err != nil {
		return fmt.Errorf("failed to get index item: %w", err)
	}

	indexItem := GspatialjpIndexItemFrom(indexItemRaw)
	log.Infofc(ctx, "geospatialjp index item: %s", pp.Sprint(indexItem))

	gdataItemRaw, err := cms.GetItem(ctx, cityItem.GeospatialjpData, true)
	if err != nil {
		return fmt.Errorf("failed to get geospatialjp data item: %w", err)
	}

	gdataItem := GspatialjpDataItemFrom(gdataItemRaw)
	log.Infofc(ctx, "geospatialjp data item: %s", pp.Sprint(gdataItem))

	if gdataItem != nil && !conf.IgnoreStatus {
		if !gdataItem.ShouldMergeCityGML() {
			log.Infofc(ctx, "skip citygml because status is running")
			conf.SkipCityGML = true
		}
		if !gdataItem.ShouldMergePlateau() {
			log.Infofc(ctx, "skip plateau because status is running")
			conf.SkipPlateau = true
		}
		if !gdataItem.ShouldMergeMaxLOD() {
			log.Infofc(ctx, "skip maxlod because status is running")
			conf.SkipMaxLOD = true
		}
	}

	if conf.SkipCityGML && conf.SkipPlateau && conf.SkipMaxLOD && conf.SkipRelated && conf.SkipIndex && !conf.ValidateMaxLOD {
		return fmt.Errorf("no command to run")
	}

	cw := &CMSWrapper{
		CMS:         cms,
		ProjectID:   conf.ProjectID,
		DataItemID:  cityItem.GeospatialjpData,
		CityItemID:  conf.CityItemID,
		SkipCityGML: conf.SkipCityGML,
		SkipPlateau: conf.SkipPlateau,
		SkipMaxLOD:  conf.SkipMaxLOD,
		SkipIndex:   conf.SkipIndex,
		WetRun:      conf.WetRun,
	}

	if cityItem.YearInt() == 0 {
		if conf.SkipImcompleteItems {
			log.Infofc(ctx, "skip because year is invalid")
			return nil
		}

		cw.Commentf(ctx, "公開準備処理を開始できません。整備年度が不正です: %s", cityItem.Year)
		return fmt.Errorf("invalid year: %s", cityItem.Year)
	}

	if cityItem.SpecVersionMajorInt() == 0 {
		if conf.SkipImcompleteItems {
			log.Infofc(ctx, "skip because spec version is invalid")
			return nil
		}

		cw.Commentf(ctx, "公開準備処理を開始できません。仕様書バージョンが不正です: %s", cityItem.Spec)
		return fmt.Errorf("invalid spec version: %s", cityItem.Spec)
	}

	if cityItem.GetUpdateCount() == 0 {
		if conf.SkipImcompleteItems {
			log.Infofc(ctx, "skip because update count is invalid")
			return nil
		}

		cw.Commentf(ctx, "公開準備処理を開始できません。codeListsのzipファイルの命名規則が不正のため版数を読み取れませんでした。もう一度ファイル名の命名規則を確認してください。_1_op_のような文字が必須です。: %s", cityItem.CodeLists)
		return fmt.Errorf("invalid update count: %s", cityItem.CodeLists)
	}

	tmpDirName := fmt.Sprintf("%s-%d", time.Now().Format("20060102-150405"), rand.Intn(1000))
	tmpDir := filepath.Join(tmpDirBase, tmpDirName)
	_ = os.MkdirAll(tmpDir, 0755)
	log.Infofc(ctx, "tmp dir: %s", tmpDir)

	if conf.Clean {
		defer func() {
			log.Infofc(ctx, "cleaning up tmp dir...: %s", tmpDir)
			if err := os.RemoveAll(tmpDir); err != nil {
				log.Warnf("failed to remove tmp dir: %s", err)
			}
		}()
	}

	log.Infofc(ctx, "getting all feature items...")
	allFeatureItems, err := getAllFeatureItems(ctx, cms, cityItem)
	if err != nil {
		cw.NotifyError(ctx, err, !conf.SkipCityGML, !conf.SkipPlateau, !conf.SkipMaxLOD)
		return fmt.Errorf("failed to get all feature items: %w", err)
	}

	log.Infofc(ctx, "feature items: %s", pp.Sprint(allFeatureItems))

	dic := mergeDics(allFeatureItems)
	log.Infofc(ctx, "dic: %s", pp.Sprint(dic))

	mc := MergeContext{
		TmpDir:             tmpDir,
		CityItem:           cityItem,
		AllFeatureItems:    allFeatureItems,
		GspatialjpDataItem: gdataItem,
		WetRun:             conf.WetRun,
		FeatureTypes:       conf.FeatureTypes,
	}

	cw.NotifyRunning(ctx)

	// maxlod
	if !conf.SkipMaxLOD {
		if err := PrepareMaxLOD(ctx, cw, mc); err != nil {
			return err
		}
	} else if conf.ValidateMaxLOD {
		if err := ValidateMaxLOD(ctx, cw, mc); err != nil {
			return err
		}
	}

	var citygmlPath, plateauPath, relatedPath string

	// related
	if !conf.SkipRelated {
		res, err := PrepareRelated(ctx, cw, mc)
		if err != nil {
			return err
		}

		relatedPath = res
	}

	if relatedPath == "" && !conf.SkipIndex && gdataItem.RelatedURL != "" {
		// download zip
		relatedPath, err = downloadFileTo(ctx, gdataItem.RelatedURL, tmpDir)
		if err != nil {
			return fmt.Errorf("failed to download merged related: %w", err)
		}
	}

	// citygml
	if !conf.SkipCityGML {
		res, err := PrepareCityGML(ctx, cw, mc)
		if err != nil {
			return err
		}

		citygmlPath = res
	}

	if citygmlPath == "" && !conf.SkipIndex && gdataItem.CityGMLURL != "" {
		// download zip
		citygmlPath, err = downloadFileTo(ctx, gdataItem.CityGMLURL, tmpDir)
		if err != nil {
			return fmt.Errorf("failed to download merged citygml: %w", err)
		}
	}

	// plateau
	if !conf.SkipPlateau {
		res, w, err := PreparePlateau(ctx, cw, mc)
		if err != nil {
			return err
		}

		if len(w) > 0 {
			cw.Comment(ctx, "公開準備処理中に警告が発生しました：\n"+strings.Join(w, "\n"))
		}

		plateauPath = res
	}

	if plateauPath == "" && !conf.SkipIndex && gdataItem.PlateauURL != "" {
		// download zip
		plateauPath, err = downloadFileTo(ctx, gdataItem.PlateauURL, tmpDir)
		if err != nil {
			return fmt.Errorf("failed to download merged plateau: %w", err)
		}
	}

	log.Infofc(ctx, "citygml path: %s", citygmlPath)
	log.Infofc(ctx, "plateau path: %s", plateauPath)

	if !conf.SkipIndex && citygmlPath != "" && plateauPath != "" {
		if err := PrepareIndex(ctx, cw, &IndexSeed{
			CityName:       cityItem.CityName,
			CityCode:       cityItem.CityCode,
			Year:           cityItem.YearInt(),
			V:              cityItem.SpecVersionMajorInt(),
			CityGMLZipPath: citygmlPath,
			PlateuaZipPath: plateauPath,
			RelatedZipPath: relatedPath,
			Generic:        indexItem.Generic,
			Dic:            dic,
		}, conf.FeatureTypes); err != nil {
			return err
		}
	} else {
		log.Infofc(ctx, "skip index")
	}

	cw.Comment(ctx, "公開準備処理が完了しました。")
	log.Infofc(ctx, "done")
	return
}
