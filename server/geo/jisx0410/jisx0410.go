// Package jisx0410 provides functions to convert JIS X 0410 Square Grid Codes
// into geographic bounds. The JIS X 0410 is a standardized grid-based coding
// system used in Japan for statistical and geographical data analysis.
// This package includes functions to validate and parse these codes and to
// convert them into latitude and longitude bounds.
package jisx0410

import (
	"fmt"

	"github.com/JamesLMilner/quadtree-go"
	"github.com/eukarya-inc/reearth-plateauview/server/geo"
)

const (
	degree = 3600 * 8
	lv1wi  = 3600 * 8
	lv1hi  = 2400 * 8
	lv2wi  = lv1wi / 8
	lv2hi  = lv1hi / 8
	lv3wi  = lv2wi / 10
	lv3hi  = lv2hi / 10
	lv3w2i = lv3wi * 2
	lv3h2i = lv3hi * 2
	lv3w5i = lv3wi * 5
	lv3h5i = lv3hi * 5
	lv3whi = lv3wi / 2
	lv3hhi = lv3hi / 2
	lv3wqi = lv3wi / 4
	lv3hqi = lv3hi / 4
	lv3wei = lv3wi / 8
	lv3hei = lv3hi / 8

	lv1w  = 1.0 * lv1wi / degree
	lv1h  = 1.0 * lv1hi / degree
	lv2w  = 1.0 * lv2wi / degree
	lv2h  = 1.0 * lv2hi / degree
	lv3w  = 1.0 * lv3wi / degree
	lv3h  = 1.0 * lv3hi / degree
	lv3w2 = 1.0 * lv3w2i / degree
	lv3h2 = 1.0 * lv3h2i / degree
	lv3w5 = 1.0 * lv3w5i / degree
	lv3h5 = 1.0 * lv3h5i / degree
	lv3wh = 1.0 * lv3whi / degree
	lv3hh = 1.0 * lv3hhi / degree
	lv3wq = 1.0 * lv3wqi / degree
	lv3hq = 1.0 * lv3hqi / degree
	lv3we = 1.0 * lv3wei / degree
	lv3he = 1.0 * lv3hei / degree
)

// digits must be unsigned and in the range 0-9 for numbers,
// otherwise, they must be odd numbers greater than or equal to 10.
// There is code that relies on the behavior of 0-1 becoming 255.
// There is code that checks for even numbers, and characters other than [0-9] must be judged as odd.
var digits [256]uint8

func init() {
	for i := range digits {
		digits[i] = 11
	}
	for i := range 10 {
		digits['0'+i] = uint8(i)
	}
}

type MeshCode struct {
	Level  int
	Bounds geo.Bounds2
}

func (m MeshCode) IsValid() bool {
	return isValidLevel(m.Level) && m.Bounds != geo.Bounds2{}
}

func (m MeshCode) String() string {
	lat := m.Bounds.Max.Y // 上側緯度
	lon := m.Bounds.Min.X // 左側経度

	// 緯度経度（秒単位）に変換
	latSec := int(lat * degree)
	lonSec := int(lon * degree)

	// 第1次メッシュ
	p1 := latSec / 2400 // 緯度 1度ごと
	p2 := lonSec / 3600 // 経度 1度ごと
	code := fmt.Sprintf("%02d%02d", p1, p2)

	if m.Level == 1 {
		return code
	}

	// 第2次メッシュ
	latRem1 := latSec % 2400
	lonRem1 := lonSec % 3600
	p3 := latRem1 / 300 // 第2次: 緯度方向8分割
	p4 := lonRem1 / 450 // 第2次: 経度方向8分割
	code += fmt.Sprintf("%d%d", p3, p4)

	if m.Level == 2 {
		return code
	}

	// 第3次メッシュ
	latRem2 := latRem1 % 300
	lonRem2 := lonRem1 % 450
	p5 := latRem2 / 30 // 第3次: 緯度方向10分割
	p6 := lonRem2 / 45 // 第3次: 経度方向10分割
	code += fmt.Sprintf("%d%d", p5, p6)

	if m.Level == 3 {
		return code
	}

	// 以下は第3次より細かいメッシュ（500m, 250m, 125m）
	latRem3 := latRem2 % 30
	lonRem3 := lonRem2 % 45

	switch m.Level {
	case 9: // 500m: 第3次を2x2分割
		p7 := (latRem3/15)*2 + (lonRem3 / 22)
		code += fmt.Sprintf("%d", p7)
	case 10: // 250m: 4x4
		p7 := float64(latRem3) / 7.5
		p8 := float64(lonRem3) / 11.25
		code += fmt.Sprintf("%d", int(p7)*4+int(p8))
	case 11: // 125m: 8x8
		p7 := float64(latRem3) / 3.75
		p8 := float64(lonRem3) / 5.625
		code += fmt.Sprintf("%d", int(p7)*8+int(p8))
	}

	return code
}

// Parse converts JIS X 0410 Square Grid Code to MeshCode and returns them.
func Parse(s string) (MeshCode, error) {
	var zero MeshCode
	if len(s) < 4 || len(s) == 5 || 11 < len(s) {
		return zero, fmt.Errorf("invalid length: %d", len(s))
	}
	c3 := digits[s[3]]
	c0 := digits[s[0]]
	c1 := digits[s[1]]
	c2 := digits[s[2]]
	if c0 > 9 || c1 > 9 || c2 > 9 || c3 > 9 {
		return zero, invalidChar(s, over(9, c0, c1, c2, c3))
	}
	lat := int32(c0*10+c1) * lv1hi
	lng := int32(c2*10+c3+100) * lv1wi
	if len(s) == 4 {
		return MeshCode{
			Level: 1,
			Bounds: geo.ToBounds2(quadtree.Bounds{
				X:      float64(lng) / degree,
				Y:      float64(lat) / degree,
				Width:  lv1w,
				Height: lv1h,
			}),
		}, nil
	}
	c5 := digits[s[5]]
	c4 := digits[s[4]]
	if c4 > 7 || c5 > 7 {
		return zero, invalidChar(s, 4+over(7, c4, c5))
	}
	lat += int32(c4) * lv2hi
	lng += int32(c5) * lv2wi
	if len(s) == 6 {
		return MeshCode{
			Level: 2,
			Bounds: geo.ToBounds2(quadtree.Bounds{
				X:      float64(lng) / degree,
				Y:      float64(lat) / degree,
				Width:  lv2w,
				Height: lv2h,
			}),
		}, nil
	}
	if len(s) == 7 {
		x := digits[s[6]] - 1
		if x > 3 {
			return zero, invalidChar(s, 6)
		}
		if x&1 != 0 {
			lng += lv3w5i
		}
		if x&2 != 0 {
			lat += lv3h5i
		}
		return MeshCode{
			Level: 0,
			Bounds: geo.ToBounds2(quadtree.Bounds{
				X:      float64(lng) / degree,
				Y:      float64(lat) / degree,
				Width:  lv3w5,
				Height: lv3h5,
			}),
		}, nil
	}
	if len(s) == 9 && s[8] == '5' {
		c6 := digits[s[6]]
		c7 := digits[s[7]]
		if c6&1 != 0 || c7&1 != 0 {
			return zero, invalidChar(s, 6+over(0, c6&1, c7&1))
		}
		lat += int32(c6>>1) * lv3h2i // [02468]
		lng += int32(c7>>1) * lv3w2i // [02468]
		return MeshCode{
			Level: 0,
			Bounds: geo.ToBounds2(quadtree.Bounds{
				X:      float64(lng) / degree,
				Y:      float64(lat) / degree,
				Width:  lv3w2,
				Height: lv3h2,
			}),
		}, nil
	}
	c7 := digits[s[7]]
	c6 := digits[s[6]]
	if c6 > 9 || c7 > 9 {
		return zero, invalidChar(s, 6+over(9, c6, c7))
	}
	lat += int32(c6) * lv3hi
	lng += int32(c7) * lv3wi
	if len(s) == 8 {
		return MeshCode{
			Level: 3,
			Bounds: geo.ToBounds2(quadtree.Bounds{
				X:      float64(lng) / degree,
				Y:      float64(lat) / degree,
				Width:  lv3w,
				Height: lv3h,
			}),
		}, nil
	}
	{
		x := digits[s[8]] - 1
		if x > 3 {
			return zero, invalidChar(s, 8)
		}
		if x&1 != 0 {
			lng += lv3whi
		}
		if x&2 != 0 {
			lat += lv3hhi
		}
	}
	if len(s) == 9 {
		return MeshCode{
			Level: 4,
			Bounds: geo.ToBounds2(quadtree.Bounds{
				X:      float64(lng) / degree,
				Y:      float64(lat) / degree,
				Width:  lv3wh,
				Height: lv3hh,
			}),
		}, nil
	}
	{
		x := digits[s[9]] - 1
		if x > 3 {
			return zero, invalidChar(s, 9)
		}
		if x&1 != 0 {
			lng += lv3wqi
		}
		if x&2 != 0 {
			lat += lv3hqi
		}
	}
	if len(s) == 10 {
		return MeshCode{
			Level: 5,
			Bounds: geo.ToBounds2(quadtree.Bounds{
				X:      float64(lng) / degree,
				Y:      float64(lat) / degree,
				Width:  lv3wq,
				Height: lv3hq,
			}),
		}, nil
	}
	{
		x := digits[s[10]] - 1
		if x > 3 {
			return zero, invalidChar(s, 10)
		}
		if x&1 != 0 {
			lng += lv3wei
		}
		if x&2 != 0 {
			lat += lv3hei
		}
	}
	return MeshCode{
		Level: 6,
		Bounds: geo.ToBounds2(quadtree.Bounds{
			X:      float64(lng) / degree,
			Y:      float64(lat) / degree,
			Width:  lv3we,
			Height: lv3he,
		}),
	}, nil
}

// TODO: fix this
func Find(lon, lat float64, level int) MeshCode {
	if !isValidLevel(level) {
		return MeshCode{}
	}

	latSec := int(lat * 3600)
	lonSec := int(lon * 3600)

	// 第1次
	p1 := latSec / 2400
	p2 := lonSec / 3600
	minLat := float64(p1*2400) / 3600
	minLon := float64(p2*3600) / 3600
	h := 40.0 / 60
	w := 1.0

	if level == 1 {
		return MeshCode{Level: 1, Bounds: geo.Bounds2{Min: geo.Point2{X: minLon, Y: minLat}, Max: geo.Point2{X: minLon + w, Y: minLat + h}}}
	}

	// 第2次
	latRem := latSec % 2400
	lonRem := lonSec % 3600
	p3 := latRem / 300
	p4 := lonRem / 450
	minLat += float64(p3*300) / 3600
	minLon += float64(p4*450) / 3600
	h = 5.0 / 60
	w = 7.5 / 60

	if level == 2 {
		return MeshCode{Level: 2, Bounds: geo.Bounds2{Min: geo.Point2{X: minLon, Y: minLat}, Max: geo.Point2{X: minLon + w, Y: minLat + h}}}
	}

	// 第3次
	latRem = latRem % 300
	lonRem = lonRem % 450
	p5 := latRem / 30
	p6 := lonRem / 45
	minLat += float64(p5*30) / 3600
	minLon += float64(p6*45) / 3600
	h = 30.0 / 3600
	w = 45.0 / 3600

	return MeshCode{
		Level: level,
		Bounds: geo.Bounds2{
			Min: geo.Point2{X: minLon, Y: minLat},
			Max: geo.Point2{X: minLon + w, Y: minLat + h},
		},
	}
}

func FindAll(minLon, minLat, maxLon, maxLat float64, level int) []MeshCode {
	stepLon := lv1w
	stepLat := lv1h

	switch level {
	case 2:
		stepLon = lv2w
		stepLat = lv2h
	case 3:
		stepLon = lv3w
		stepLat = lv3h
	case 9:
		stepLon = lv3w2
		stepLat = lv3h2
	case 10:
		stepLon = lv3wq
		stepLat = lv3hq
	case 11:
		stepLon = lv3we
		stepLat = lv3he
	}

	codes := []MeshCode{}
	for lat := maxLat; lat >= minLat; lat -= stepLat {
		for lon := minLon; lon <= maxLon; lon += stepLon {
			code := Find(lon, lat, level)
			codes = append(codes, code)
		}
	}
	return dedupe(codes)
}

func over(x uint8, c ...uint8) int {
	for i := range c {
		if c[i] > x {
			return i
		}
	}
	panic("unreachable")
}

func invalidChar(s string, idx int) error {
	return fmt.Errorf("invalid char(idx=%d): %c", idx, s[idx])
}

func dedupe(codes []MeshCode) []MeshCode {
	seen := make(map[string]bool)
	result := []MeshCode{}
	for _, code := range codes {
		key := code.String()
		if !seen[key] {
			seen[key] = true
			result = append(result, code)
		}
	}
	return result
}

func isValidLevel(level int) bool {
	return level >= 1 && level <= 3
}
