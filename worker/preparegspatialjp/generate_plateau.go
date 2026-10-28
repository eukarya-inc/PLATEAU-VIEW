package preparegspatialjp

import (
	"archive/zip"
	"context"
	"fmt"
	"os"
	"path"
	"path/filepath"
	"strings"

	"github.com/dustin/go-humanize"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/PLATEAU-VIEW/worker/workerutil"
	"github.com/reearth/reearthx/log"
)

func PreparePlateau(ctx context.Context, c *CMSWrapper, m MergeContext) (res string, warning []string, err error) {
	defer func() {
		if err == nil {
			return
		}
		err = fmt.Errorf("3D Tiles,MVTのマージに失敗しました: %w", err)
		c.NotifyError(ctx, err, false, true, false)
	}()

	path, warning, err := mergePlateau(ctx, m, false)
	if err != nil {
		err = fmt.Errorf("failed to prepare plateau: %w", err)
		return
	}

	aid, err := c.UploadFile(ctx, path)
	if err != nil {
		err = fmt.Errorf("failed to upload file: %w", err)
		return
	}

	// the JGD2024 version is made only when the 3D Tiles have been converted into JGD2024
	var aidJGD2024 string
	if hasJGD2024Data(m) {
		pathJGD2024, w, err2 := mergePlateau(ctx, m, true)
		warning = append(warning, w...)
		if err2 != nil {
			err = fmt.Errorf("failed to prepare plateau (JGD2024): %w", err2)
			return
		}

		aidJGD2024, err = c.UploadFile(ctx, pathJGD2024)
		if err != nil {
			err = fmt.Errorf("failed to upload file: %w", err)
			return
		}
	}

	if err2 := c.UpdateDataItem(ctx, &GspatialjpDataItem{
		MergePlateauStatus: successTag,
		Plateau:            aid,
		PlateauJGD2024:     aidJGD2024,
	}); err2 != nil {
		err = fmt.Errorf("failed to update data item: %w", err2)
		return
	}

	res = path
	log.Infofc(ctx, "plateau prepared: %s", path)
	return
}

// hasJGD2024Data reports whether any feature item has a JGD2024 asset.
func hasJGD2024Data(m MergeContext) bool {
	for _, ft := range m.FeatureTypes {
		for _, url := range m.AllFeatureItems[ft].Data {
			if cmsintegrationcommon.IsJGD2024Asset(url) {
				return true
			}
		}
	}
	return false
}

// includeInPlateau reports whether the asset goes into the zip of the CRS. The JGD2011 zip
// has the assets that are not of JGD2024. The JGD2024 zip has the JGD2024 assets and the
// assets that are the same in every CRS (MVT).
func includeInPlateau(url string, jgd2024 bool) bool {
	if cmsintegrationcommon.IsJGD2024Asset(url) {
		return jgd2024
	}
	return !jgd2024 || isCRSIndependentAsset(url)
}

// isCRSIndependentAsset reports whether the asset has no heights, so that it is the same
// in every CRS. That is MVT, whose name has "_mvt" or "_dm_geometric_attributes".
func isCRSIndependentAsset(url string) bool {
	name := strings.TrimSuffix(path.Base(url), path.Ext(url))
	return strings.Contains(name, "_mvt") || strings.Contains(name, "_dm_geometric_attributes")
}

func mergePlateau(ctx context.Context, m MergeContext, jgd2024 bool) (string, []string, error) {
	var warning []string
	tmpDir := m.TmpDir
	allFeatureItems := m.AllFeatureItems
	suffix := ""
	if jgd2024 {
		suffix = strings.TrimPrefix(cmsintegrationcommon.JGD2024AssetSuffix, "_")
	}
	dataName := m.FileName("3dtiles_mvt", suffix)

	zipFileName := dataName + ".zip"
	zipFilePath := filepath.Join(tmpDir, zipFileName)

	log.Infofc(ctx, "preparing plateau: %s", dataName)

	f, err := os.Create(zipFilePath)
	if err != nil {
		return "", nil, fmt.Errorf("failed to create file: %w", err)
	}

	defer func() { _ = f.Close() }()

	cz := NewZip2zip(zip.NewWriter(f))
	defer func() { _ = cz.Close() }()

	for _, ft := range m.FeatureTypes {
		fi, ok := allFeatureItems[ft]
		if !ok || fi.Data == nil {
			log.Debugfc(ctx, "no data for %s", ft)
			continue
		}

		log.Infofc(ctx, "preparing plateau (%s)...", ft)

		for _, url := range fi.Data {
			log.Debugfc(ctx, "downloading url: %s", url)
			if url == "" || !includeInPlateau(url, jgd2024) {
				continue
			}

			var needRootDir bool
			rootDirName := strings.TrimSuffix(path.Base(url), path.Ext(url))
			if rootDirName == "" {
				continue
			}

			err := downloadAndConsumeZip(ctx, url, tmpDir, func(zr *zip.Reader, fi os.FileInfo) error {
				log.Debugfc(ctx, "donwloaded %s (%s)", url, humanize.Bytes(uint64(fi.Size())))

				return cz.Run(zr, func(f *zip.File) (string, error) {
					p := workerutil.NormalizeZipFilePath(f.Name)
					if p == "" {
						return "", nil
					}

					rootDir, _, found := strings.Cut(p, "/")
					if !found || rootDir != rootDirName {
						needRootDir = true
						p = path.Join(rootDirName, p)
					}

					log.Debugfc(ctx, "zipping %s -> %s", f.Name, p)
					return p, nil
				})
			})
			if needRootDir {
				warning = append(warning, fmt.Sprintf("zipファイル内にルートディレクトリ \"%s\" が必要です: %s", rootDirName, path.Base(url)))
			}
			if err != nil {
				return "", warning, fmt.Errorf("failed to download and consume zip: %w", err)
			}
		}
	}

	return zipFilePath, warning, nil
}
