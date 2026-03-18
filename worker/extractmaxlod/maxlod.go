package extractmaxlod

import (
	"archive/zip"
	"path"
	"strconv"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/worker/workerutil"
	"github.com/reearth/reearthx/log"
)

type MaxLOD struct {
	Code   string
	Type   string
	MaxLod string
	File   string
}

func extractMaxLOD(zr *zip.Reader, ft, maxlod string) (res []MaxLOD) {
	for _, f := range zr.File {
		fileName := path.Base(workerutil.NormalizeZipFilePath(f.Name))
		if path.Ext(fileName) != ".gml" {
			continue
		}

		parts := strings.Split(fileName, "_")
		if len(parts) < 3 {
			log.Warnf("invalid file name: %s", fileName)
			continue
		}

		if ft != "" && parts[1] != ft {
			log.Infof("invalid type (%s): %s", ft, fileName)
			continue
		}

		if _, err := strconv.Atoi(parts[0]); err != nil {
			log.Warnf("invalid code: %s", fileName)
			continue
		}

		log.Infof("%s (code:%s, type:%s, maxlod:%s)", fileName, parts[0], parts[1], maxlod)
		res = append(res, MaxLOD{
			Code:   parts[0],
			Type:   parts[1],
			MaxLod: maxlod,
			File:   fileName,
		})
	}

	return
}

func maxlodCSV(maxlod []MaxLOD) string {
	var b strings.Builder
	le := len(maxlod)
	b.WriteString("code,type,max_lod,file\n")
	for i, m := range maxlod {
		isLast := i == le-1
		b.WriteString(m.Code)
		b.WriteString(",")
		b.WriteString(m.Type)
		b.WriteString(",")
		b.WriteString(m.MaxLod)
		b.WriteString(",")
		b.WriteString(m.File)
		if !isLast {
			b.WriteString("\n")
		}
	}
	return b.String()
}
