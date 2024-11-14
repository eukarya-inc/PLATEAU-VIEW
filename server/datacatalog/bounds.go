package datacatalog

import (
	"fmt"
	"strconv"
	"strings"

	"github.com/JamesLMilner/quadtree-go"
)

func parseBounds(s string) (quadtree.Bounds, error) {
	tokens := strings.Split(s, ",")
	if !(len(tokens) == 2 || len(tokens) == 4) {
		return quadtree.Bounds{}, fmt.Errorf("invalid bounds")
	}

	fs := make([]float64, 0, len(tokens))
	for _, t := range tokens {
		f, err := strconv.ParseFloat(t, 64)
		if err != nil {
			return quadtree.Bounds{}, fmt.Errorf("invalid bounds: %s", t)
		}
		fs = append(fs, f)
	}

	if len(fs) == 2 {
		return quadtree.Bounds{
			X: fs[0],
			Y: fs[1],
		}, nil
	}

	if fs[0] >= fs[2] || fs[1] >= fs[3] {
		return quadtree.Bounds{}, fmt.Errorf("invalid bounds: x2 and y2 must be greater than x1 and y1")
	}

	return quadtree.Bounds{
		X:      fs[0],
		Y:      fs[1],
		Width:  fs[2] - fs[0],
		Height: fs[3] - fs[1],
	}, nil
}

func intersects(a, b quadtree.Bounds) bool {
	aMaxX := a.X + a.Width
	aMaxY := a.Y + a.Height
	bMaxX := b.X + b.Width
	bMaxY := b.Y + b.Height

	// a is left of b
	if aMaxX <= b.X {
		return false
	}
	// a is right of b
	if a.X >= bMaxX {
		return false
	}
	// a is above b
	if aMaxY <= b.Y {
		return false
	}
	// a is below b
	if a.Y >= bMaxY {
		return false
	}
	// The two overlap
	return true
}
