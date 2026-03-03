package geospatialjpv3

import (
	"context"
	"fmt"
	"regexp"
	"strconv"
	"strings"

	"github.com/dustin/go-humanize"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/ckan"
	"github.com/k0kubun/pp/v3"
	"github.com/reearth/reearthx/log"
	"github.com/samber/lo"
)

func (h *handler) Publish(ctx context.Context, cityItem *CityItem) (err error) {
	cms := h.cms

	defer func() {
		if err != nil {
			errmsg := err.Error()
			comment := fmt.Sprintf("G空間情報センターのデータセットの公開に失敗しました: %s", errmsg)

			if err2 := cms.CommentToItem(ctx, cityItem.ID, comment); err2 != nil {
				log.Errorfc(ctx, "geospatialjpv3: failed to comment to city item: %v", err2)
			}

			if err2 := cms.CommentToItem(ctx, cityItem.GeospatialjpData, comment); err2 != nil {
				log.Errorfc(ctx, "geospatialjpv3: failed to comment to data item: %v", err2)
			}
		}
	}()

	log.Infofc(ctx, "geospatialjpv3: publish")

	seed, err := getSeed(ctx, cms, cityItem, h.ckanOrg)
	if err != nil {
		return fmt.Errorf("failed to get seed: %w", err)
	}

	log.Debugfc(ctx, "geospatialjpv3: seed: %s", pp.Sprint(seed))
	if !seed.Valid() {
		return fmt.Errorf("アップロード可能なアイテムがありません。")
	}

	pkgSeed := PackageSeedFrom(cityItem, seed)

	pkg, pkgCreated, err := h.createOrUpdatePackage(ctx, pkgSeed)
	if err != nil {
		return fmt.Errorf("%s: %w", errMsgPackageSearchCreateFailed, err)
	}

	log.Debugfc(ctx, "geospatialjpv3: pkg: %s", pp.Sprint(pkg))
	resources := []ckan.Resource{}
	var resourceErrors []string

	if seed.Index != "" {
		log.Debugfc(ctx, "geospatialjpv3: index: %s", seed.Index)
		r, err := h.createOrUpdateResource(ctx, pkg, ResourceInfo{
			Name:        fmt.Sprintf("データ目録（v%d）", seed.V),
			URL:         seed.IndexURL,
			Description: seed.Index,
		})
		if err != nil {
			log.Errorfc(ctx, "geospatialjpv3: failed to create or update resource (index): %v", err)
			resourceErrors = append(resourceErrors, fmt.Sprintf("データ目録: %v", err))
		} else {
			resources = append(resources, r)
		}
	}

	if seed.IndexMapURL != "" {
		log.Debugfc(ctx, "geospatialjpv3: index map: %s", seed.IndexMapURL)
		r, err := h.createOrUpdateResource(ctx, pkg, ResourceInfo{
			Name:        fmt.Sprintf("索引図（v%d）", seed.V),
			URL:         seed.IndexMapURL,
			Description: "データ整備範囲の標準地域メッシュ（２次メッシュ、３次メッシュ）のメッシュとメッシュ番号を示したPDFファイルです。",
		})
		if err != nil {
			log.Errorfc(ctx, "geospatialjpv3: %s: %v", errMsgResourceCreateIndexFailed, err)
			resourceErrors = append(resourceErrors, fmt.Sprintf("索引図: %v", err))
		} else {
			resources = append(resources, r)
		}
	}

	if seed.CityGML != "" {
		log.Debugfc(ctx, "geospatialjpv3: citygml: %s", seed.CityGML)
		r, err := h.createOrUpdateResource(ctx, pkg, ResourceInfo{
			Name:        fmt.Sprintf("CityGML（v%d）", seed.V),
			URL:         seed.CityGML,
			Description: seed.CityGMLDescription,
		})
		if err != nil {
			log.Errorfc(ctx, "geospatialjpv3: %s: %v", errMsgResourceCreateCityGMLFailed, err)
			resourceErrors = append(resourceErrors, fmt.Sprintf("CityGML: %v", err))
		} else {
			resources = append(resources, r)
		}
	}

	if seed.Plateau != "" {
		log.Debugfc(ctx, "geospatialjpv3: plateau: %s", seed.Plateau)
		r, err := h.createOrUpdateResource(ctx, pkg, ResourceInfo{
			Name:        fmt.Sprintf("3D Tiles, MVT（v%d）", seed.V),
			URL:         seed.Plateau,
			Description: seed.PlateauDescription,
		})
		if err != nil {
			log.Errorfc(ctx, "geospatialjpv3: %s: %v", errMsgResourceCreate3DTilesFailed, err)
			resourceErrors = append(resourceErrors, fmt.Sprintf("3D Tiles, MVT: %v", err))
		} else {
			resources = append(resources, r)
		}
	}

	if seed.Related != "" {
		log.Debugfc(ctx, "geospatialjpv3: related: %s", seed.Related)
		r, err := h.createOrUpdateResource(ctx, pkg, ResourceInfo{
			Name:        fmt.Sprintf(("関連データセット（v%d）"), seed.V),
			URL:         seed.Related,
			Description: seed.RelatedDescription,
		})
		if err != nil {
			log.Errorfc(ctx, "geospatialjpv3: %s: %v", errMsgResourceCreateRelatedFailed, err)
			resourceErrors = append(resourceErrors, fmt.Sprintf("関連データセット: %v", err))
		} else {
			resources = append(resources, r)
		}
	}

	if seed.Generics != nil {
		log.Debugfc(ctx, "geospatialjpv3: generics: %s", pp.Sprint(seed.Generics))
		for _, g := range seed.Generics {
			if g.Asset == nil || g.Asset.URL == "" {
				continue
			}

			url := g.Asset.URL
			if g.Name == "" {
				log.Errorfc(ctx, "geospatialjpv3: その他データセットの名前は必須です。: %#v", g)
				resourceErrors = append(resourceErrors, "その他データセット: 名前は必須です")
				continue
			}

			size := g.Asset.TotalSize
			if size == 0 {
				log.Errorfc(ctx, "geospatialjpv3: その他データセットのアセットサイズを正しく取得できませんでした。: %#v", g)
				resourceErrors = append(resourceErrors, fmt.Sprintf("その他データセット(%s): アセットサイズを正しく取得できませんでした", g.Name))
				continue
			}

			r, err := h.createOrUpdateResource(ctx, pkg, ResourceInfo{
				Name:        g.Name,
				URL:         url,
				Description: replaceSize(g.Desc, uint64(size)),
			})
			if err != nil {
				log.Errorfc(ctx, "geospatialjpv3: %s (%s): %v", errMsgResourceCreateOtherFailed, g.Name, err)
				resourceErrors = append(resourceErrors, fmt.Sprintf("その他データセット(%s): %v", g.Name, err))
				continue
			}
			resources = append(resources, r)
		}
	}

	if len(resources) > 0 {
		log.Debugfc(ctx, "geospatialjpv3: reorder: %v", resources)
		resourceIDs := lo.Map(resources, func(r ckan.Resource, _ int) string {
			return r.ID
		})

		if err := h.reorderResources(ctx, pkg.ID, resourceIDs); err != nil {
			log.Errorfc(ctx, "geospatialjpv3: %s: %v", errMsgResourceReorderPartialSuccess, err)
			resourceErrors = append(resourceErrors, fmt.Sprintf("リソースの並び替え: %v", err))
		}
	}

	var comment string
	if pkgCreated {
		comment = fmt.Sprintf("G空間情報センターにデータセットを新規作成しました。 \n%s", h.packageURL(pkg))
	} else {
		comment = fmt.Sprintf("G空間情報センターのデータセットを更新しました。 \n%s", h.packageURL(pkg))
	}

	if len(resourceErrors) > 0 {
		comment += "\n\n以下のリソースの登録に失敗しました:\n" + strings.Join(resourceErrors, "\n")
	}

	if err := h.cms.CommentToItem(ctx, seed.GspatialjpDataItemID, comment); err != nil {
		log.Errorfc(ctx, "geospatialjpv3: failed to comment to data item: %v", err)
	}

	if err := h.cms.CommentToItem(ctx, cityItem.ID, comment); err != nil {
		log.Errorfc(ctx, "geospatialjpv3: failed to comment to city item: %v", err)
	}

	return nil
}

func (h *handler) packageURL(pkg *ckan.Package) string {
	return fmt.Sprintf("%s/dataset/%s", strings.TrimSuffix(h.ckanBase, "/"), pkg.Name)
}

var reResourceVersion = regexp.MustCompile(`(?:\(|（)v(\d+)(?:\)|）)$`)

func extractVersionFromResourceName(name string) *int {
	m := reResourceVersion.FindStringSubmatch(name)
	if len(m) < 2 {
		return nil
	}

	i, err := strconv.Atoi(m[1])
	if err != nil {
		return nil
	}

	return &i
}

var reSize = regexp.MustCompile(`\${{.*_?SIZE *}}`)

func replaceSize(s string, size uint64) string {
	return reSize.ReplaceAllString(s, humanize.Bytes(size))
}
