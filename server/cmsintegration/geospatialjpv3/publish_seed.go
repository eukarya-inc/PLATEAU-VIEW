package geospatialjpv3

import (
	"bytes"
	"context"
	"fmt"
	"net/http"
	"strings"

	"github.com/k0kubun/pp/v3"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/log"
	"github.com/vincent-petithory/dataurl"
)

type Seed struct {
	CityGML              string
	Plateau              string
	Related              string
	CityGMLSize          uint64
	PlateauSize          uint64
	RelatedSize          uint64
	Desc                 string
	Index                string
	IndexURL             string
	CityGMLDescription   string
	PlateauDescription   string
	RelatedDescription   string
	Area                 string
	ThumbnailURL         string `pp:"-"`
	IndexMapURL          string
	Author               string
	AuthorEmail          string
	Maintainer           string
	MaintainerEmail      string
	Quality              string
	SpecVersion          string
	V                    int
	Year                 int
	Generics             []CMSGenericItem
	GspatialjpDataItemID string
	Org                  string
}

func (s Seed) Valid() bool {
	return s.CityGML != "" || s.Plateau != "" || s.Related != ""
}

func getSeed(ctx context.Context, c cms.Interface, cityItem *CityItem, org string) (seed Seed, err error) {
	seed.Org = org

	rawDataItem, err := c.GetItem(ctx, cityItem.GeospatialjpData, true)
	if err != nil {
		return seed, fmt.Errorf("G空間センター用データアイテムが取得できません: %w", err)
	}

	rawIndexItem, err := c.GetItem(ctx, cityItem.GeospatialjpIndex, true)
	if err != nil {
		return seed, fmt.Errorf("G空間センター用目録アイテムが取得できません: %w", err)
	}

	var dataItem CMSDataItem
	rawDataItem.Unmarshal(&dataItem)

	var indexItem CMSIndexItem
	rawIndexItem.Unmarshal(&indexItem)

	log.Debugfc(ctx, "geospatialjpv3: rawDataItem: %s", pp.Sprint(rawDataItem))
	log.Debugfc(ctx, "geospatialjpv3: rawIndexItem: %s", pp.Sprint(rawIndexItem))
	log.Debugfc(ctx, "geospatialjpv3: dataItem: %s", pp.Sprint(dataItem))
	log.Debugfc(ctx, "geospatialjpv3: indexItem: %s", pp.Sprint(indexItem))

	if indexItem.Thumbnail != nil && indexItem.Thumbnail.URL != "" {
		seed.ThumbnailURL, err = fetchAndGetDataURL(indexItem.Thumbnail.URL)
		if err != nil {
			return seed, fmt.Errorf("サムネイルが取得できませんでした: %w", err)
		}
	}

	seed.GspatialjpDataItemID = dataItem.ID
	if dataItem.CityGML != nil {
		seed.CityGML = dataItem.CityGML.URL
		seed.CityGMLSize = dataItem.CityGML.TotalSize
	}
	if dataItem.Plateau != nil {
		seed.Plateau = dataItem.Plateau.URL
		seed.PlateauSize = dataItem.Plateau.TotalSize
	}
	if dataItem.Related != nil {
		seed.Related = dataItem.Related.URL
		seed.RelatedSize = dataItem.Related.TotalSize
	}
	if indexItem.IndexMap != nil {
		seed.IndexMapURL = indexItem.IndexMap.URL
	}
	if indexItem.Desc != "" {
		seed.Desc = indexItem.Desc
	}

	seed.Index = indexItem.DescIndex
	if seed.Index == "" {
		seed.Index = dataItem.DescIndex
	}
	if indexItem.IndexData != nil {
		seed.IndexURL = indexItem.IndexData.URL
	}
	if seed.Index != "" && seed.IndexURL == "" {
		seed.IndexURL = dataurl.New([]byte(seed.Index), "text/markdown").String()
	}

	seed.CityGMLDescription = replaceSize(indexItem.DescCityGML, seed.CityGMLSize)
	seed.PlateauDescription = replaceSize(indexItem.DescPlateau, seed.PlateauSize)
	seed.RelatedDescription = replaceSize(indexItem.DescRelated, seed.RelatedSize)
	seed.Area = indexItem.Region
	if seed.Area == "" && cityItem != nil && cityItem.Prefecture != "" && cityItem.CityName != "" {
		seed.Area = fmt.Sprintf("%s_%s", cityItem.Prefecture, cityItem.CityName)
	}
	seed.Author = indexItem.Author
	seed.AuthorEmail = indexItem.AuthorEmail
	seed.Maintainer = indexItem.Maintainer
	seed.MaintainerEmail = indexItem.MaintainerEmail
	seed.Quality = indexItem.Quality
	seed.Generics = indexItem.Generics
	seed.Year = cityItem.YearInt()
	seed.V = cityItem.SpecVersionMajorInt()
	seed.SpecVersion = cityItem.SpecVersionFull()

	return seed, nil
}

func fetchAndGetDataURL(url string) (string, error) {
	res, err := http.Get(url)
	if err != nil {
		return "", err
	}

	if res.StatusCode != http.StatusOK {
		return "", fmt.Errorf("サムネイルの取得に失敗しました。ステータスコード: %s", res.Status)
	}

	buf := bytes.NewBuffer(nil)
	if _, err := buf.ReadFrom(res.Body); err != nil {
		return "", err
	}

	data := buf.Bytes()
	mediaType := http.DetectContentType(data)
	if !strings.HasPrefix(mediaType, "image/") {
		return "", fmt.Errorf("サムネイルは正しい画像ファイルではないようです")
	}

	return dataurl.New(data, mediaType).String(), nil
}
