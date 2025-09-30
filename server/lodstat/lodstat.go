package lodstat

import "github.com/eukarya-inc/PLATEAU-VIEW/server/geo/jisx0410"

type lodstatContext struct {
	Codes     map[string]jisx0410.MeshCode
	FileSize  map[string]int64
	Features  map[string]int
	LodStat   map[string]int
	Maxlod    map[string]int
	Lod0Count map[string]int
	Lod1Count map[string]int
	Lod2Count map[string]int
	Lod3Count map[string]int
	Lod4Count map[string]int
}

func newLodstatContext() *lodstatContext {
	return &lodstatContext{
		Codes:     map[string]jisx0410.MeshCode{},
		FileSize:  map[string]int64{},
		Features:  map[string]int{},
		LodStat:   map[string]int{},
		Maxlod:    map[string]int{},
		Lod0Count: map[string]int{},
		Lod1Count: map[string]int{},
		Lod2Count: map[string]int{},
		Lod3Count: map[string]int{},
		Lod4Count: map[string]int{},
	}
}

func (c *lodstatContext) CollectAll(level int, featureType string, cityFiles []DatasetFilesResponse) {
	for _, cityFile := range cityFiles {
		c.Collect(level, featureType, cityFile)
	}
}

func (c *lodstatContext) Collect(level int, featureType string, cityFile DatasetFilesResponse) {
	for ft, gmlFiles := range cityFile {
		if featureType != "all" && ft != featureType {
			continue
		}
		for _, file := range gmlFiles {
			mesh, err := jisx0410.Parse(file.Code)
			if err != nil {
				continue
			}

			// Use the mesh code as-is if it matches the requested level
			code := file.Code

			// Handle mesh level conversion
			if level == 2 && mesh.Level == 3 {
				// If requesting level 2 but data is level 3, aggregate to level 2
				if len(code) >= 6 {
					code = code[:6] // Truncate 3rd mesh code to 2nd mesh code
					// Re-parse to get the correct bounds for level 2
					mesh, err = jisx0410.Parse(code)
					if err != nil {
						continue
					}
				}
			} else if level == 3 && mesh.Level == 2 {
				// If requesting level 3 but data is level 2, use level 2 data as-is
				// The mesh bounds will be larger but we keep the original code
				// This allows feature types with only 2nd mesh data to be displayed at level 3
			} else if mesh.Level != level {
				// Skip if level doesn't match and not handling conversion
				continue
			}

			if _, ok := c.Codes[code]; !ok {
				c.Codes[code] = mesh
			}
			if file.FileSize > 0 {
				c.FileSize[code] += file.FileSize
			}
			if file.Features > 0 {
				c.Features[code] += file.Features
			}
			switch file.MaxLod {
			case 0:
				c.LodStat[code] |= 0b00001
			case 1:
				c.LodStat[code] |= 0b00011
			case 2:
				c.LodStat[code] |= 0b00111
			case 3:
				c.LodStat[code] |= 0b01111
			case 4:
				c.LodStat[code] |= 0b11111
			}
			if lod := file.LOD0; lod != nil && *lod > 0 {
				c.Lod0Count[code] += *lod
			}
			if lod := file.LOD1; lod != nil && *lod > 0 {
				c.Lod1Count[code] += *lod
			}
			if lod := file.LOD2; lod != nil && *lod > 0 {
				c.Lod2Count[code] += *lod
			}
			if lod := file.LOD3; lod != nil && *lod > 0 {
				c.Lod3Count[code] += *lod
			}
			if lod := file.LOD4; lod != nil && *lod > 0 {
				c.Lod4Count[code] += *lod
			}
			if m, ok := c.Maxlod[code]; !ok || file.MaxLod > m {
				c.Maxlod[code] = file.MaxLod
			}
		}
	}
}

func (c *lodstatContext) Properties(code, featureType string) map[string]any {
	props := map[string]any{}
	if featureType != "all" {
		props["featureType"] = featureType
	}
	m, ok := c.Codes[code]
	if !ok {
		return nil
	}
	props["meshCode"] = code
	props["level"] = m.Level
	props["fileSize"] = c.FileSize[code]
	props["features"] = c.Features[code]
	props["maxLod"] = c.Maxlod[code]
	if lod, ok := c.LodStat[code]; ok {
		props["lod0"] = (lod & 0b00001) != 0
		props["lod1"] = (lod & 0b00010) != 0
		props["lod2"] = (lod & 0b00100) != 0
		props["lod3"] = (lod & 0b01000) != 0
		props["lod4"] = (lod & 0b10000) != 0
	}
	lod0Count := c.Lod0Count[code]
	lod1Count := c.Lod1Count[code]
	lod2Count := c.Lod2Count[code]
	lod3Count := c.Lod3Count[code]
	lod4Count := c.Lod4Count[code]
	props["lod0Count"] = lod0Count
	props["lod1Count"] = lod1Count
	props["lod2Count"] = lod2Count
	props["lod3Count"] = lod3Count
	props["lod4Count"] = lod4Count
	return props
}
