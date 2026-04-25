package mvttilejson

// TileJSON is the subset of TileJSON 3.0.0 fields emitted by Build.
// See https://github.com/mapbox/tilejson-spec/tree/master/3.0.0
type TileJSON struct {
	TileJSON     string        `json:"tilejson"`
	Name         string        `json:"name,omitempty"`
	Description  string        `json:"description,omitempty"`
	Scheme       string        `json:"scheme"`
	Tiles        []string      `json:"tiles"`
	MinZoom      int           `json:"minzoom"`
	MaxZoom      int           `json:"maxzoom"`
	Attribution  string        `json:"attribution,omitempty"`
	VectorLayers []VectorLayer `json:"vector_layers"`
	Bounds       []float64     `json:"bounds,omitempty"`
}

type VectorLayer struct {
	ID     string            `json:"id"`
	Fields map[string]string `json:"fields"`
}

const (
	defaultMinZoom = 10
	defaultMaxZoom = 16
	plateauAttr    = `<a href="https://www.mlit.go.jp/plateau/">国土交通省 PLATEAU</a>`
)

// Build assembles a TileJSON document from the matched dataset.
func Build(d Input) TileJSON {
	layers := make([]VectorLayer, 0, len(d.Layers))
	for _, l := range d.Layers {
		if l == "" {
			continue
		}
		layers = append(layers, VectorLayer{ID: l, Fields: map[string]string{}})
	}
	if len(layers) == 0 {
		// vector_layers is required by the spec; use the type code as a
		// best-effort layer id when the dataset omits explicit layer names.
		layers = append(layers, VectorLayer{ID: d.TypeCode, Fields: map[string]string{}})
	}

	return TileJSON{
		TileJSON:     "3.0.0",
		Name:         d.Name,
		Scheme:       "xyz",
		Tiles:        []string{d.URL},
		MinZoom:      defaultMinZoom,
		MaxZoom:      defaultMaxZoom,
		Attribution:  plateauAttr,
		VectorLayers: layers,
	}
}
