package estat

import (
	"encoding/binary"
	"fmt"
	"math"
	"strconv"
)

// unpackGeometry decodes a geometry packed by the former geo service.
// Firestore cannot store nested arrays, so each ring is stored as bytes of
// little-endian float64 pairs, and each array level as a map of
// {"0": item, "1": item, ..., "length": n}.
func unpackGeometry(packed map[string]any) (*Geometry, error) {
	typ, _ := packed["type"].(string)
	coords, ok := packed["coordinates"].(map[string]any)
	if !ok {
		return nil, fmt.Errorf("invalid geometry coordinates: %T", packed["coordinates"])
	}

	switch typ {
	case "Polygon":
		rings, err := mapArrayObject(coords, unpackRing)
		if err != nil {
			return nil, err
		}
		return &Geometry{Type: typ, Coordinates: rings}, nil
	case "MultiPolygon":
		polygons, err := mapArrayObject(coords, func(v any) ([][][2]float64, error) {
			polygon, ok := v.(map[string]any)
			if !ok {
				return nil, fmt.Errorf("invalid polygon: %T", v)
			}
			return mapArrayObject(polygon, unpackRing)
		})
		if err != nil {
			return nil, err
		}
		return &Geometry{Type: typ, Coordinates: polygons}, nil
	default:
		return nil, fmt.Errorf("unsupported geometry type: %q", typ)
	}
}

func mapArrayObject[T any](object map[string]any, f func(any) (T, error)) ([]T, error) {
	length, ok := toInt64(object["length"])
	if !ok || length < 0 {
		return nil, fmt.Errorf("invalid array object length: %v", object["length"])
	}
	res := make([]T, 0, length)
	for i := range length {
		item, err := f(object[strconv.FormatInt(i, 10)])
		if err != nil {
			return nil, fmt.Errorf("item %d: %w", i, err)
		}
		res = append(res, item)
	}
	return res, nil
}

// toInt64 accepts both integer and double values, since the Node.js client
// may have stored a JavaScript number as either.
func toInt64(v any) (int64, bool) {
	switch n := v.(type) {
	case int64:
		return n, true
	case float64:
		if n != math.Trunc(n) {
			return 0, false
		}
		return int64(n), true
	}
	return 0, false
}

func unpackRing(v any) ([][2]float64, error) {
	b, ok := v.([]byte)
	if !ok {
		return nil, fmt.Errorf("invalid ring: %T", v)
	}
	if len(b)%16 != 0 {
		return nil, fmt.Errorf("ring byte length must be a multiple of 16: %d", len(b))
	}
	ring := make([][2]float64, len(b)/16)
	for i := range ring {
		ring[i] = [2]float64{
			math.Float64frombits(binary.LittleEndian.Uint64(b[i*16:])),
			math.Float64frombits(binary.LittleEndian.Uint64(b[i*16+8:])),
		}
	}
	return ring, nil
}
