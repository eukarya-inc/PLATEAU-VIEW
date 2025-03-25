package preparegspatialjp

import (
	"archive/zip"

	"github.com/spf13/afero"
	"github.com/spf13/afero/zipfs"
)

func Example_generatePlateauIndexItem() {
	zipPath := ""
	featureTypes := []string{"bldg"}

	if zipPath == "" {
		return
	}

	seed := &IndexSeed{}
	name := "name"
	size := uint64(1024 * 1024 * 1024) // 1GB
	zr, err := zip.OpenReader(zipPath)
	if err != nil {
		panic(err)
	}

	f := afero.NewIOFS(zipfs.New(&zr.Reader))

	res, err := generatePlateauIndexItem(seed, name, size, f, featureTypes)
	if err != nil {
		panic(err)
	}

	text := renderIndexItem(res, 0)
	println(text)

	// Output:
}
