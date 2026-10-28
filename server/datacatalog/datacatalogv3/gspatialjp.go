package datacatalogv3

import (
	"context"
	"fmt"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/ckan"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/reearth/reearthx/log"
)

// DefaultGspatialjpBaseURL is the CKAN of G空間情報センター.
const DefaultGspatialjpBaseURL = "https://www.geospatial.jp/ckan"

const (
	// gspatialjpOrg is the CKAN organization that holds the PLATEAU datasets.
	gspatialjpOrg = "toshi"
	// gspatialjpRows is the page size of package_search. All PLATEAU packages
	// currently fit in one page.
	gspatialjpRows = 1000
	// gspatialjpTimeout bounds the whole fetch including retries.
	gspatialjpTimeout = 2 * time.Minute
	// gspatialjpTTL lets the projects built in one run (one process) share
	// a single fetch. It is shorter than the datacatalog regeneration cycle.
	gspatialjpTTL = 3 * time.Minute
)

// Gspatialjp fetches the PLATEAU datasets published on G空間情報センター.
// The result is memoised for gspatialjpTTL, including failures.
type Gspatialjp struct {
	baseURL string
	client  gspatialjpCKAN
	now     func() time.Time

	mu        sync.Mutex
	fetchedAt time.Time
	pkgs      *GspatialjpPackages
	err       error
}

type gspatialjpCKAN interface {
	SearchPackageResources(ctx context.Context, q ckan.SearchPackagesQuery) (ckan.List[ckan.PackageResources], error)
	ShowPackage(ctx context.Context, id string) (ckan.Package, error)
}

var sharedGspatialjp sync.Map // base URL -> *Gspatialjp

// SharedGspatialjp returns the process-wide Gspatialjp for the CKAN at
// baseURL (DefaultGspatialjpBaseURL when empty), so that every Repos in a
// process shares one fetch.
func SharedGspatialjp(baseURL string) (*Gspatialjp, error) {
	baseURL = strings.TrimSuffix(baseURL, "/")
	if baseURL == "" {
		baseURL = DefaultGspatialjpBaseURL
	}
	if g, ok := sharedGspatialjp.Load(baseURL); ok {
		return g.(*Gspatialjp), nil
	}

	g, err := NewGspatialjp(baseURL)
	if err != nil {
		return nil, err
	}
	actual, _ := sharedGspatialjp.LoadOrStore(baseURL, g)
	return actual.(*Gspatialjp), nil
}

func NewGspatialjp(baseURL string) (*Gspatialjp, error) {
	baseURL = strings.TrimSuffix(baseURL, "/")
	c, err := ckan.New(baseURL, "")
	if err != nil {
		return nil, fmt.Errorf("invalid G空間情報センター URL: %w", err)
	}
	return &Gspatialjp{baseURL: baseURL, client: c, now: time.Now}, nil
}

// Packages returns the PLATEAU city datasets on G空間情報センター.
func (g *Gspatialjp) Packages(ctx context.Context) (*GspatialjpPackages, error) {
	g.mu.Lock()
	defer g.mu.Unlock()

	if !g.fetchedAt.IsZero() && g.now().Sub(g.fetchedAt) < gspatialjpTTL {
		return g.pkgs, g.err
	}

	// the result is shared with every caller, so the first caller's cancellation must not become everyone's error
	ctx, cancel := context.WithTimeout(context.WithoutCancel(ctx), gspatialjpTimeout)
	defer cancel()

	t := time.Now()
	g.pkgs, g.err = g.fetch(ctx)
	g.fetchedAt = g.now()
	if g.err == nil {
		log.Infofc(ctx, "datacatalogv3: fetched %d G空間情報センター datasets in %.1fs", len(g.pkgs.packages), time.Since(t).Seconds())
	}
	return g.pkgs, g.err
}

func (g *Gspatialjp) fetch(ctx context.Context) (*GspatialjpPackages, error) {
	res := &GspatialjpPackages{
		baseURL:  g.baseURL,
		packages: map[gspatialjpKey]*GspatialjpPackage{},
	}

	for start := 0; ; {
		l, err := g.client.SearchPackageResources(ctx, ckan.SearchPackagesQuery{
			FQ:    "organization:" + gspatialjpOrg,
			Sort:  "id asc", // name is not sortable on geospatial.jp
			Rows:  gspatialjpRows,
			Start: start,
		})
		if err != nil {
			return nil, err
		}

		for _, p := range l.Results {
			key, ok := gspatialjpKeyFromPackageName(p.Name)
			if !ok {
				continue
			}

			resources, ok := p.ResourceList()
			if !ok {
				full, err := g.client.ShowPackage(ctx, p.Name)
				if err != nil {
					res.Warnings = append(res.Warnings, fmt.Sprintf("gspatialjp: failed to get dataset %s: %v", p.Name, err))
					continue
				}
				resources = full.Resources
			}

			res.packages[key] = &GspatialjpPackage{Name: p.Name, Resources: resources}
		}

		start += len(l.Results)
		if len(l.Results) == 0 || start >= l.Count {
			break
		}
	}

	return res, nil
}

// GspatialjpPackages holds the PLATEAU city datasets on G空間情報センター.
type GspatialjpPackages struct {
	baseURL  string
	packages map[gspatialjpKey]*GspatialjpPackage
	// Warnings lists datasets that could not be read.
	Warnings []string
}

// GspatialjpPackage is a dataset (CKAN package) on G空間情報センター.
type GspatialjpPackage struct {
	Name      string
	Resources []ckan.Resource
}

type gspatialjpKey struct {
	cityCode string
	year     int
}

// The city datasets are named plateau-<city code>-<city name>-<year> (see
// geospatialjpv3). Only the code and year are compared, as the name part has
// several spellings (e.g. -shi and -city).
var (
	reGspatialjpPackage          = regexp.MustCompile(`^plateau-(\d{5})-[0-9a-z-]+-(\d{4})$`)
	reGspatialjpPackageTokyo23ku = regexp.MustCompile(`^plateau-tokyo23ku-(\d{4})$`)
)

const tokyo23kuCode = "13100"

func gspatialjpKeyFromPackageName(name string) (gspatialjpKey, bool) {
	code, year := "", ""
	if m := reGspatialjpPackage.FindStringSubmatch(name); m != nil {
		code, year = m[1], m[2]
	} else if m := reGspatialjpPackageTokyo23ku.FindStringSubmatch(name); m != nil {
		code, year = tokyo23kuCode, m[1]
	} else {
		return gspatialjpKey{}, false
	}

	y, err := strconv.Atoi(year)
	if err != nil {
		return gspatialjpKey{}, false
	}
	return gspatialjpKey{cityCode: code, year: y}, true
}

// NewGspatialjpPackages builds GspatialjpPackages from packages, skipping
// those not named like a city dataset. It is meant for tests.
func NewGspatialjpPackages(baseURL string, pkgs ...*GspatialjpPackage) *GspatialjpPackages {
	res := &GspatialjpPackages{
		baseURL:  strings.TrimSuffix(baseURL, "/"),
		packages: map[gspatialjpKey]*GspatialjpPackage{},
	}
	for _, p := range pkgs {
		if key, ok := gspatialjpKeyFromPackageName(p.Name); ok {
			res.packages[key] = p
		}
	}
	return res
}

// Find returns the dataset of the city and year, or nil.
func (p *GspatialjpPackages) Find(cityCode string, year int) *GspatialjpPackage {
	if p == nil {
		return nil
	}
	return p.packages[gspatialjpKey{cityCode: cityCode, year: year}]
}

// DatasetURL returns the dataset page URL of pkg.
func (p *GspatialjpPackages) DatasetURL(pkg *GspatialjpPackage) string {
	return p.baseURL + "/dataset/" + pkg.Name
}

// reGspatialjpResourceVersion matches the spec major version suffix that
// geospatialjpv3 appends to resource names, e.g. "CityGML（v5）". Older
// resources use half-width parentheses or "V".
var reGspatialjpResourceVersion = regexp.MustCompile(`\s*[（(][vV](\d+)[）)]$`)

// gspatialjpExcludedResources are not exposed in the data catalog.
var gspatialjpExcludedResources = map[string]bool{
	"データ目録": true,
	"索引図":   true,
	"図郭マップ": true,
}

// gspatialjpV1Resources are the resources of the first (2020) release, which
// have no version suffix. They belong to spec v1, which no data catalog
// entry uses.
var gspatialjpV1Resources = map[string]bool{
	"CityGML":                       true,
	"3D Tiles, GeoJson, MVT, Shape": true,
	"FBX":                           true,
	"OBJ":                           true,
	"GeoTIFF":                       true,
	"ファイルジオデータベース":                  true,
	"ファイルジオデータベース（防災）": true,
}

// gspatialjpFiles lists the files of pkg for a CityGML entry of the given
// spec major version, in the order of the dataset page.
//
//   - "CityGML（vN）", "3D Tiles, MVT（vN）" and "関連データセット（vN）" become
//     citygml, 3dtiles_mvt and related.
//   - Any other file becomes other, e.g. "GeoTIFF（vN）" and その他データセット
//     (which have no version suffix).
//   - Resources of another spec version, データ目録, 索引図 and data URIs are
//     skipped.
//
// sizes maps CMS asset URLs to their size; files with another URL have no
// size.
func gspatialjpFiles(pkg *GspatialjpPackage, specMajor int, sizes map[string]int64) []*plateauapi.GspatialjpFile {
	if pkg == nil {
		return nil
	}

	var res []*plateauapi.GspatialjpFile
	for _, r := range pkg.Resources {
		url := strings.TrimSpace(r.URL)
		if url == "" || strings.HasPrefix(url, "data:") {
			continue
		}

		name := strings.TrimSpace(r.Name)
		base, version := name, 0
		if m := reGspatialjpResourceVersion.FindStringSubmatchIndex(name); m != nil {
			base = name[:m[0]]
			version, _ = strconv.Atoi(name[m[2]:m[3]])
		}

		if version == 0 && gspatialjpV1Resources[base] {
			version = 1
		}
		if version != 0 && version != specMajor || gspatialjpExcludedResources[base] {
			continue
		}

		kind := plateauapi.GspatialjpFileKindOther
		if version != 0 {
			switch base {
			case "CityGML":
				kind = plateauapi.GspatialjpFileKindCityGML
			case "3D Tiles, MVT":
				kind = plateauapi.GspatialjpFileKind3DTilesMVT
			case "関連データセット":
				kind = plateauapi.GspatialjpFileKindRelated
			}
		}

		res = append(res, &plateauapi.GspatialjpFile{
			Kind:     kind,
			Name:     name,
			URL:      url,
			FileSize: sizes[url],
		})
	}
	return res
}

// specMajorVersion returns the major version of a spec such as "第4.1版".
func specMajorVersion(spec string) int {
	major, _, _ := strings.Cut(plateauapi.SpecNumber(spec), ".")
	v, _ := strconv.Atoi(major)
	return v
}
