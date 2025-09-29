package lodstat

import "github.com/eukarya-inc/PLATEAU-VIEW/server/geo/jisx0410"

type lodstatContext struct {
	Features  map[string]jisx0410.MeshCode
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
		Features:  map[string]jisx0410.MeshCode{},
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
			if mesh.Level != level {
				continue
			}
			if _, ok := c.Features[file.Code]; !ok {
				c.Features[file.Code] = mesh
			}
			switch file.MaxLod {
			case 0:
				c.LodStat[file.Code] |= 0b00001
			case 1:
				c.LodStat[file.Code] |= 0b00011
			case 2:
				c.LodStat[file.Code] |= 0b00111
			case 3:
				c.LodStat[file.Code] |= 0b01111
			case 4:
				c.LodStat[file.Code] |= 0b11111
			}
			if lod := file.LOD0; lod != nil && *lod > 0 {
				c.Lod0Count[file.Code] += *lod
			}
			if lod := file.LOD1; lod != nil && *lod > 0 {
				c.Lod1Count[file.Code] += *lod
			}
			if lod := file.LOD2; lod != nil && *lod > 0 {
				c.Lod2Count[file.Code] += *lod
			}
			if lod := file.LOD3; lod != nil && *lod > 0 {
				c.Lod3Count[file.Code] += *lod
			}
			if lod := file.LOD4; lod != nil && *lod > 0 {
				c.Lod4Count[file.Code] += *lod
			}
			if m, ok := c.Maxlod[file.Code]; !ok || file.MaxLod > m {
				c.Maxlod[file.Code] = file.MaxLod
			}
		}
	}
}

func (c *lodstatContext) Properties(code, featureType string) map[string]any {
	props := map[string]any{}
	if featureType != "all" {
		props["featureType"] = featureType
	}
	props["meshCode"] = code
	props["maxLod"] = c.Maxlod[code]
	if lod, ok := c.LodStat[code]; ok {
		props["lod0"] = (lod & 0b00001) != 0
		props["lod1"] = (lod & 0b00010) != 0
		props["lod2"] = (lod & 0b00100) != 0
		props["lod3"] = (lod & 0b01000) != 0
		props["lod4"] = (lod & 0b10000) != 0
	}
	props["lod0Count"] = c.Lod0Count[code]
	props["lod1Count"] = c.Lod1Count[code]
	props["lod2Count"] = c.Lod2Count[code]
	props["lod3Count"] = c.Lod3Count[code]
	props["lod4Count"] = c.Lod4Count[code]
	return props
}
