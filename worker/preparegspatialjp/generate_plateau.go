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
	"github.com/eukarya-inc/reearth-plateauview/worker/workerutil"
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

	path, warning, err := mergePlateau(ctx, m)
	if err != nil {
		err = fmt.Errorf("failed to prepare plateau: %w", err)
		return
	}

	aid, err := c.UploadFile(ctx, path)
	if err != nil {
		err = fmt.Errorf("failed to upload file: %w", err)
		return
	}

	if err2 := c.UpdateDataItem(ctx, &GspatialjpDataItem{
		MergePlateauStatus: successTag,
		Plateau:            aid,
	}); err2 != nil {
		err = fmt.Errorf("failed to update data item: %w", err2)
		return
	}

	res = path
	log.Infofc(ctx, "plateau prepared: %s", path)
	return
}

func mergePlateau(ctx context.Context, m MergeContext) (string, []string, error) {
	var warning []string
	tmpDir := m.TmpDir
	allFeatureItems := m.AllFeatureItems
	dataName := m.FileName("3dtiles_mvt", "")

	zipFileName := dataName + ".zip"
	zipFilePath := filepath.Join(tmpDir, zipFileName)

	log.Infofc(ctx, "preparing plateau: %s", dataName)

	f, err := os.Create(zipFilePath)
	if err != nil {
		return "", nil, fmt.Errorf("failed to create file: %w", err)
	}

	defer f.Close()

	cz := NewZip2zip(zip.NewWriter(f))
	defer cz.Close()

	for _, ft := range m.FeatureTypes {
		fi, ok := allFeatureItems[ft]
		if !ok || fi.Data == nil {
			log.Debugfc(ctx, "no data for %s", ft)
			continue
		}

		log.Infofc(ctx, "preparing plateau (%s)...", ft)

		for _, url := range fi.Data {
			log.Debugfc(ctx, "downloading url: %s", url)
			if url == "" {
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
