package datacatalogv3

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"

	"github.com/samber/lo"
)

type AssetName struct {
	CityCode    string
	CityName    string
	Provider    string
	Year        int
	Format      string
	UpdateCount int
	Option      string // オプション（_opの前に来る任意の文字列）
	Ex          AssetNameEx
}

func (n AssetName) String() string {
	var option string
	if n.Option != "" {
		option = "_" + n.Option
	}
	var ex string
	if n.Ex.Ex != "" {
		ex = "_" + n.Ex.Ex
	}
	return fmt.Sprintf("%s_%s_%s_%d_%s_%d%s_op%s", n.CityCode, n.CityName, n.Provider, n.Year, n.Format, n.UpdateCount, option, ex)
}

type AssetNameEx struct {
	Normal *AssetNameExNormal
	Fld    *AssetNameExFld
	Ex     string
}

func (ex AssetNameEx) String() string {
	return ex.Ex
}

func (ex AssetNameEx) IsValid() bool {
	return ex.Normal != nil || ex.Fld != nil
}

func (ex AssetNameEx) DatasetItemKey() string {
	switch {
	case ex.Normal != nil:
		return ex.Normal.DatasetItemKey()
	case ex.Fld != nil:
		return ex.Fld.DatasetItemKey()
	}
	return ""
}

func (ex AssetNameEx) DatasetKey() string {
	switch {
	case ex.Normal != nil:
		return ex.Normal.DatasetKey()
	case ex.Fld != nil:
		return ex.Fld.DatasetKey()
	}
	return ""
}

func (ex AssetNameEx) DicKey() string {
	switch {
	case ex.Normal != nil:
		return ex.Normal.DicKey()
	case ex.Fld != nil:
		return ex.Fld.DicKey()
	}
	return ""
}

type AssetNameExNormal struct {
	Type      string
	Name      string
	Format    string
	WardCode  string
	WardName  string
	LOD       int
	LODEx     int
	NoTexture bool
	NoLOD     bool
	Interior  bool
}

func (ex AssetNameExNormal) DatasetItemKey() string {
	return ex.Name
}

func (ex AssetNameExNormal) DatasetKey() string {
	return ex.Name
}

func (ex AssetNameExNormal) DicKey() string {
	return ex.Name
}

type AssetNameExFld struct {
	Type      string
	Admin     string
	River     string
	Format    string
	L         int
	Suffix    string
	NoTexture bool
}

func (ex AssetNameExFld) DatasetItemKey() string {
	return fmt.Sprintf("l%d", ex.L)
}

func (ex AssetNameExFld) DatasetKey() string {
	return fmt.Sprintf("%s_%s%s", ex.Admin, ex.River, ex.suffix("-"))
}

func (ex AssetNameExFld) DicKey() string {
	return fmt.Sprintf("%s_l%d%s", ex.River, ex.L, ex.suffix("-"))
}

func (ex AssetNameExFld) suffix(sep string) string {
	suffix := ""
	if ex.Suffix != "" {
		suffix = sep + ex.Suffix
	}
	return suffix
}

var reAssetName = regexp.MustCompile(`^(\d+)_([a-z0-9-]+)_([a-z0-9-]+)_(\d{4})_([a-z]+)_(\d+)(.*)$`)

func ParseAssetName(name string) *AssetName {
	m := reAssetName.FindStringSubmatch(name)
	if len(m) == 0 {
		return nil
	}

	year, _ := strconv.Atoi(m[4])
	updateCount, _ := strconv.Atoi(m[6])

	// Parse suffix to extract option and extension
	// Format: [_option][_op][_extension]
	option, ex := parseOptionAndExtension(m[7])

	return &AssetName{
		CityCode:    m[1],
		CityName:    m[2],
		Provider:    m[3],
		Year:        year,
		Format:      m[5],
		UpdateCount: updateCount,
		Option:      option,
		Ex:          ParseAssetNameEx(ex),
	}
}

// parseOptionAndExtension parses the suffix after update count
// and extracts option (before _op) and extension (after _op)
func parseOptionAndExtension(suffix string) (option, ex string) {
	if suffix == "" {
		return "", ""
	}

	// Remove leading underscore
	suffix = strings.TrimPrefix(suffix, "_")
	if suffix == "" {
		return "", ""
	}

	// Case 1: Just "op" (_op only, no option, no extension)
	if suffix == "op" {
		return "", ""
	}

	// Case 2: Starts with "op_" (_op followed by extension)
	if strings.HasPrefix(suffix, "op_") {
		return "", strings.TrimPrefix(suffix, "op_")
	}

	// Case 3: Ends with "_op" (option followed by _op, no extension)
	if strings.HasSuffix(suffix, "_op") {
		return strings.TrimSuffix(suffix, "_op"), ""
	}

	// Case 4: Contains "_op_" (option + _op + extension)
	if idx := strings.Index(suffix, "_op_"); idx != -1 {
		return suffix[:idx], suffix[idx+4:]
	}

	// Case 5: No _op found, treat as extension only (backward compatibility)
	return "", suffix
}

func ParseAssetNameEx(name string) (ex AssetNameEx) {
	ex.Ex = name

	ex.Fld = ParseAssetNameExFld(name)
	if ex.Fld != nil {
		return
	}

	// ex.Urf = ParseAssetNameExUrf(name)
	// if ex.Urf != nil {
	// 	return
	// }

	ex.Normal = ParseAssetNameExNormal(name)
	return
}

var reAssetNameExNormal = regexp.MustCompile(`^([a-z]+)(?:_([A-Za-z0-9-_]+))?_(mvt|3dtiles|dm_geometric_attributes)(?:_(\d+)_([a-z0-9-]+))?(_lod\d\d?)?(_interior)?(_no_texture)?$`)

func ParseAssetNameExNormal(name string) *AssetNameExNormal {
	if name == "" {
		return nil
	}

	m := reAssetNameExNormal.FindStringSubmatch(name)
	if len(m) == 0 {
		return nil
	}

	if m[3] == "dm_geometric_attributes" {
		m[3] = "mvt"
		if m[6] == "" {
			m[6] = "0"
		}
	}

	nolod := false
	lod := 0
	lodex := 0
	if m[6] != "" {
		lods := strings.TrimPrefix(m[6], "_lod")
		if len(lods) == 2 {
			lodex, _ = strconv.Atoi(lods[1:])
			lods = lods[:1]
		}
		lod, _ = strconv.Atoi(lods)
	} else {
		nolod = true
	}

	return &AssetNameExNormal{
		Type:      m[1],
		Name:      m[2],
		Format:    m[3],
		WardCode:  m[4],
		WardName:  m[5],
		LOD:       lod,
		LODEx:     lodex,
		Interior:  m[7] != "",
		NoTexture: m[8] != "",
		NoLOD:     nolod,
	}
}

var reAssetNameExFld = regexp.MustCompile(`^fld_(natl|pref)_([A-Za-z0-9-_]+)_3dtiles_(l\d+)(?:-(.+?))?(_no_texture)?$`)

func ParseAssetNameExFld(name string) *AssetNameExFld {
	if name == "" {
		return nil
	}

	m := reAssetNameExFld.FindStringSubmatch(name)
	if len(m) == 0 {
		return nil
	}

	l, _ := strconv.Atoi(m[3][1:])

	return &AssetNameExFld{
		Type:      "fld",
		Admin:     m[1],
		River:     m[2],
		Format:    "3dtiles",
		L:         l,
		Suffix:    m[4],
		NoTexture: m[5] != "",
	}
}

func ParseAssetUrls(urls []string) []*AssetName {
	return lo.Map(urls, func(u string, _ int) *AssetName {
		return ParseAssetName(nameWithoutExt(nameFromURL(u)))
	})
}

type RelatedAssetName struct {
	Code     string
	Name     string
	Year     int
	Provider string
	WardCode string
	WardName string
	Type     string
	Format   string
}

var reRelatedAssetName = regexp.MustCompile(`^(\d+)_([a-zA-Z0-9-]+)_([a-zA-Z0-9-]+)_(\d+)_(?:(\d+)_([a-zA-Z0-9-]+?)_)?([a-zA-Z0-9-_]+)\.([a-z0-9]+)$`)

func ParseRelatedAssetName(name string) *RelatedAssetName {
	if name == "" {
		return nil
	}

	m := reRelatedAssetName.FindStringSubmatch(name)
	if m == nil {
		return nil
	}

	y, _ := strconv.Atoi(m[4])
	return &RelatedAssetName{
		Code:     m[1],
		Name:     m[2],
		Provider: m[3],
		Year:     y,
		WardCode: m[5],
		WardName: m[6],
		Type:     m[7],
		Format:   m[8],
	}
}
