package jisx0410mvt

import (
	"math"

	"github.com/eukarya-inc/reearth-plateauview/server/geo/jisx0410"
	"github.com/tidwall/mvt"
)

func RenderMVTTile(z, x, y, level int, layername string) ([]byte, error) {
	// get meshes
	tileMinLon, tileMinLat, tileMaxLon, tileMaxLat := tileBounds(z, x, y)
	meshecodes := jisx0410.FindAll(
		tileMinLon, tileMinLat,
		tileMaxLon, tileMaxLat,
		level,
	)

	if len(meshecodes) == 0 {
		return nil, nil
	}

	// render tile
	return renderMVTTile(z, x, y, layername, meshecodes)
}

const tileSize = 256

func tileBounds(z, x, y int) (minLon, minLat, maxLon, maxLat float64) {
	n := math.Exp2(float64(z))
	minLon = float64(x)/n*360.0 - 180.0
	maxLon = float64(x+1)/n*360.0 - 180.0

	latRadMin := math.Atan(math.Sinh(math.Pi * (1 - 2*float64(y+1)/n)))
	latRadMax := math.Atan(math.Sinh(math.Pi * (1 - 2*float64(y)/n)))

	minLat = latRadMin * 180.0 / math.Pi
	maxLat = latRadMax * 180.0 / math.Pi
	return
}

func latLonToTilePixel(lat, lon float64, z, tileX, tileY int) (float64, float64) {
	sinLat := math.Sin(lat * math.Pi / 180.0)
	n := math.Exp2(float64(z)) * tileSize

	pixelX := (lon + 180.0) / 360.0 * n
	pixelY := (0.5 - math.Log((1+sinLat)/(1-sinLat))/(4*math.Pi)) * n

	return pixelX - float64(tileX)*tileSize, pixelY - float64(tileY)*tileSize
}

func renderMVTTile(z, x, y int, layername string, meshcodes []jisx0410.MeshCode) ([]byte, error) {
	tileMinLon, tileMinLat, tileMaxLon, tileMaxLat := tileBounds(z, x, y)

	var tile mvt.Tile
	layer := tile.AddLayer(layername)

	count := 0
	for _, mesh := range meshcodes {
		if !mesh.IsValid() {
			continue
		}

		bbox := mesh.Bounds
		minLon, minLat, maxLon, maxLat := bbox.Min.X, bbox.Min.Y, bbox.Max.X, bbox.Max.Y

		if maxLon < tileMinLon || minLon > tileMaxLon || maxLat < tileMinLat || minLat > tileMaxLat {
			continue
		}

		x0, y0 := latLonToTilePixel(maxLat, minLon, z, x, y)
		x1, y1 := latLonToTilePixel(minLat, maxLon, z, x, y)

		feature := layer.AddFeature(mvt.Polygon)
		feature.MoveTo(x0, y0)
		feature.LineTo(x1, y0)
		feature.LineTo(x1, y1)
		feature.LineTo(x0, y1)
		feature.LineTo(x0, y0)
		feature.ClosePath()
		feature.AddTag("meshcode", mesh.String())
		feature.AddTag("level", mesh.Level)

		count++
	}

	if count == 0 {
		return nil, nil
	}

	return tile.Render(), nil
}
