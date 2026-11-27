package cmsintsetup

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"regexp"
	"slices"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintflow"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	cms "github.com/reearth/reearth-cms-api/go"
	"golang.org/x/oauth2"
)

// The JGD2024 reconversion converts again the feature items whose 3D Tiles are published, so
// that a JGD2024 version is added next to the JGD2011 one. The Flow triggers of the specs
// already output JGD2024, and the server merges the result into the item by CRS.
//
// For each city and feature type, only the item of the latest year is converted, because that
// is the one the data catalog serves. Items are started one at a time through the server's
// /reconvert, at most Concurrency at once, and each is followed until its conversion ends.

// ReconvertProject is a yearly PLATEAU project.
type ReconvertProject struct {
	ID   string `json:"id"`
	Year int    `json:"year"`
}

type ReconvertJGD2024Input struct {
	// Projects are the projects to look at. When empty, every PLATEAU project of the v3 schema
	// in the workspaces model is used.
	Projects []ReconvertProject
	// Years limits the projects found in the workspaces model. Empty means every year.
	Years []int
	// CityCodes and FeatureTypes limit the items. Empty means all.
	CityCodes    []string
	FeatureTypes []string
	// Limit is the maximum number of conversions to start. 0 means no limit.
	Limit int
	// Concurrency is the number of conversions that run at once.
	Concurrency int
	// MaxFailures stops starting new conversions once this many have not ended with the JGD2024
	// assets (failed, no JGD2024 asset or timed out). The running ones are still followed. 0 means no limit.
	MaxFailures int
	// ServerURL is the base URL of the PLATEAU VIEW server that has /reconvert. Token gives the
	// Google ID token of an allowed service account, issued for the server's URL.
	ServerURL string
	Token     oauth2.TokenSource
	// StateFile records the result of every item as JSON lines. Items recorded as done are skipped,
	// so running again with the same file resumes the work.
	StateFile    string
	PollInterval time.Duration
	// Timeout is how long one conversion is followed before it is recorded as timed out.
	Timeout time.Duration
	DryRun  bool
}

// ReconvertTarget is a feature item to convert.
type ReconvertTarget struct {
	ProjectID string `json:"projectId"`
	Year      int    `json:"year"`
	CityCode  string `json:"cityCode"`
	CityName  string `json:"cityName"`
	Model     string `json:"model"`
	ItemID    string `json:"itemId"`
	// Skip is why the item is not converted. Empty means it is converted.
	Skip string `json:"skip,omitempty"`
}

func (t ReconvertTarget) String() string {
	return fmt.Sprintf("%d %s %s %s (%s)", t.Year, t.CityCode, t.CityName, strings.TrimPrefix(t.Model, cmsintegrationcommon.ModelPrefix), t.ItemID)
}

type ReconvertResultKind string

const (
	// ReconvertDone means the item has the JGD2024 3D Tiles.
	ReconvertDone ReconvertResultKind = "done"
	// ReconvertFailed means the conversion ended with an error, or could not be started.
	ReconvertFailed ReconvertResultKind = "failed"
	// ReconvertNoJGD2024 means the conversion succeeded but no JGD2024 asset was added.
	ReconvertNoJGD2024 ReconvertResultKind = "no_jgd2024"
	// ReconvertTimeout means the conversion did not end within the timeout. It may still be running.
	ReconvertTimeout ReconvertResultKind = "timeout"
)

// ReconvertRecord is a line of the state file.
type ReconvertRecord struct {
	Time    time.Time           `json:"time"`
	Target  ReconvertTarget     `json:"target"`
	Result  ReconvertResultKind `json:"result"`
	Message string              `json:"message,omitempty"`
}

type ReconvertSummary struct {
	Targets  int                         `json:"targets"`
	Skipped  map[string]int              `json:"skipped"`
	Results  map[ReconvertResultKind]int `json:"results"`
	Failures []ReconvertRecord           `json:"failures,omitempty"`
}

func (s *ReconvertSummary) String() string {
	var b strings.Builder
	fmt.Fprintf(&b, "targets: %d", s.Targets)
	for _, k := range sortedKeys(s.Skipped) {
		fmt.Fprintf(&b, "\nskip (%s): %d", k, s.Skipped[k])
	}
	for _, k := range []ReconvertResultKind{ReconvertDone, ReconvertNoJGD2024, ReconvertFailed, ReconvertTimeout} {
		if n := s.Results[k]; n > 0 {
			fmt.Fprintf(&b, "\n%s: %d", k, n)
		}
	}
	for _, f := range s.Failures {
		fmt.Fprintf(&b, "\n%s %s: %s", f.Result, f.Target, f.Message)
	}
	return b.String()
}

const (
	skipAlreadyJGD2024 = "already has JGD2024"
	skipNoHeights      = "no 3d tiles (mvt only or no data)"
	skipNoCityGML      = "no citygml"
	skipOldSpec        = "spec older than 4.x"
	skipNotFlow        = "not converted with flow"
	skipRunning        = "qc or conversion is running"
	skipDoneInState    = "done in the state file"
	skipLimit          = "over the limit"
)

// minReconvertSpecMajor is the oldest spec that Flow has workflows for.
const minReconvertSpecMajor = 4

var plateauProjectAliasRe = regexp.MustCompile(`^plateau-(\d{4})$`)

// ReconvertJGD2024 plans the targets and, unless DryRun, converts them.
// onTarget is called for every planned target, and onRecord for every result.
func ReconvertJGD2024(ctx context.Context, s *Services, specs plateaucms.SpecStore, inp ReconvertJGD2024Input, onTarget func(ReconvertTarget), onRecord func(ReconvertRecord)) (*ReconvertSummary, error) {
	if !inp.DryRun && (inp.ServerURL == "" || inp.Token == nil) {
		return nil, errors.New("server URL and a token source are required")
	}
	inp.ServerURL = strings.TrimSuffix(inp.ServerURL, "/")
	if inp.Concurrency <= 0 {
		inp.Concurrency = 1
	}
	if inp.PollInterval <= 0 {
		inp.PollInterval = 30 * time.Second
	}
	if inp.Timeout <= 0 {
		inp.Timeout = 6 * time.Hour
	}

	projects := inp.Projects
	if len(projects) == 0 {
		var err error
		projects, err = plateauProjects(ctx, s.CMS, s.SystemProject, inp.Years)
		if err != nil {
			return nil, err
		}
	}
	if len(projects) == 0 {
		return nil, errors.New("no projects")
	}

	done, err := readReconvertState(inp.StateFile)
	if err != nil {
		return nil, err
	}

	targets, err := planReconvert(ctx, s, specs, projects, inp)
	if err != nil {
		return nil, err
	}

	sum := &ReconvertSummary{Skipped: map[string]int{}, Results: map[ReconvertResultKind]int{}}
	var run []ReconvertTarget
	for _, t := range targets {
		if t.Skip == "" && done[t.ItemID] {
			t.Skip = skipDoneInState
		}
		if t.Skip == "" && inp.Limit > 0 && len(run) >= inp.Limit {
			t.Skip = skipLimit
		}
		if onTarget != nil {
			onTarget(t)
		}
		if t.Skip != "" {
			sum.Skipped[t.Skip]++
			continue
		}
		run = append(run, t)
	}
	sum.Targets = len(run)

	if inp.DryRun || len(run) == 0 {
		return sum, nil
	}

	state, err := openReconvertState(inp.StateFile)
	if err != nil {
		return sum, err
	}
	defer func() { _ = state.Close() }()

	// the server answers once the conversion is requested to Flow
	r := &reconverter{s: s, inp: inp, http: &http.Client{Timeout: 2 * time.Minute}}
	var mu sync.Mutex
	stopCtx, stopStarting := context.WithCancel(ctx)
	defer stopStarting()
	record := func(rec ReconvertRecord) {
		mu.Lock()
		defer mu.Unlock()
		sum.Results[rec.Result]++
		if rec.Result != ReconvertDone {
			sum.Failures = append(sum.Failures, rec)
			if inp.MaxFailures > 0 && len(sum.Failures) >= inp.MaxFailures {
				stopStarting()
			}
		}
		if err := state.Write(rec); err != nil && onRecord != nil {
			onRecord(ReconvertRecord{Time: time.Now(), Target: rec.Target, Result: ReconvertFailed, Message: "failed to write the state file: " + err.Error()})
		}
		if onRecord != nil {
			onRecord(rec)
		}
	}

	queue := make(chan ReconvertTarget)
	var wg sync.WaitGroup
	for range inp.Concurrency {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for t := range queue {
				record(r.convert(ctx, t))
			}
		}()
	}

	for _, t := range run {
		select {
		case queue <- t:
		case <-stopCtx.Done():
		}
		if stopCtx.Err() != nil {
			break
		}
	}
	close(queue)
	wg.Wait()

	if err := ctx.Err(); err != nil {
		return sum, err
	}
	if stopCtx.Err() != nil {
		return sum, fmt.Errorf("stopped after %d failures", len(sum.Failures))
	}
	return sum, nil
}

// plateauProjects returns the PLATEAU projects of the v3 schema in the workspaces model.
func plateauProjects(ctx context.Context, c cms.Interface, systemProject string, years []int) ([]ReconvertProject, error) {
	items, err := c.GetItemsByKey(ctx, systemProject, "workspaces", false)
	if err != nil {
		return nil, fmt.Errorf("failed to get workspaces: %w", err)
	}

	var res []ReconvertProject
	for _, i := range items.Items {
		var m plateaucms.Metadata
		i.Unmarshal(&m)
		match := plateauProjectAliasRe.FindStringSubmatch(m.DataCatalogProjectAlias)
		if match == nil || m.ProjectID == "" || m.DataCatalogSchemaVersion != "v3" || m.Disabled {
			continue
		}
		year, _ := strconv.Atoi(match[1])
		if len(years) > 0 && !slices.Contains(years, year) {
			continue
		}
		res = append(res, ReconvertProject{ID: m.ProjectID, Year: year})
	}
	sort.Slice(res, func(i, j int) bool { return res[i].Year < res[j].Year })
	return res, nil
}

type reconvertCandidate struct {
	target ReconvertTarget
	item   *cms.Item
	city   *cmsintegrationcommon.CityItem
}

// planReconvert returns the feature item of the latest year for each city and feature type
// whose 3D Tiles are published, with the reason when it is not converted.
func planReconvert(ctx context.Context, s *Services, specStore plateaucms.SpecStore, projects []ReconvertProject, inp ReconvertJGD2024Input) ([]ReconvertTarget, error) {
	featureTypes, err := s.PCMS.PlateauFeatureTypes(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get feature types: %w", err)
	}
	specs, err := specStore.PlateauSpecs(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get plateau specs: %w", err)
	}
	codes := featureTypes.Codes()

	// latest[cityCode/featureType] is the published item of the latest year
	latest := map[string]reconvertCandidate{}
	for _, p := range projects {
		models, err := s.CMS.GetModels(ctx, p.ID)
		if err != nil {
			return nil, fmt.Errorf("failed to get models of %s: %w", p.ID, err)
		}
		modelIDs := map[string]string{}
		for _, m := range models.Models {
			modelIDs[m.Key] = m.ID
		}

		cityModel := modelIDs[cmsintegrationcommon.ModelPrefix+cmsintegrationcommon.CityModel]
		if cityModel == "" {
			continue
		}
		cities, err := listModelItems(ctx, s.CMS, cityModel, false)
		if err != nil {
			return nil, fmt.Errorf("failed to get cities of %s: %w", p.ID, err)
		}

		// city ID → city, for the published feature types only
		published := map[string]map[string]*cmsintegrationcommon.CityItem{}
		for _, c := range cities {
			city := cmsintegrationcommon.CityItemFrom(c, codes)
			city.ID = c.ID
			if city.CityCode == "" || len(inp.CityCodes) > 0 && !slices.Contains(inp.CityCodes, city.CityCode) {
				continue
			}
			for _, ft := range codes {
				if city.References[ft] == "" || !city.CityPublic && !city.Public[ft] {
					continue
				}
				if published[ft] == nil {
					published[ft] = map[string]*cmsintegrationcommon.CityItem{}
				}
				published[ft][c.ID] = city
			}
		}

		for _, ft := range featureTypes {
			if !ft.Conv || len(published[ft.Code]) == 0 || len(inp.FeatureTypes) > 0 && !slices.Contains(inp.FeatureTypes, ft.Code) {
				continue
			}
			model := cmsintegrationcommon.ModelPrefix + ft.Code
			if modelIDs[model] == "" {
				continue
			}
			// with assets, so that the names of the data assets can be read
			items, err := listModelItems(ctx, s.CMS, modelIDs[model], true)
			if err != nil {
				return nil, fmt.Errorf("failed to get %s items of %s: %w", ft.Code, p.ID, err)
			}
			for _, i := range items {
				cityID := i.FieldByKey("city").GetValue().String()
				if cityID == nil {
					continue
				}
				city := published[ft.Code][*cityID]
				if city == nil || city.References[ft.Code] != i.ID {
					continue
				}
				// the data catalog serves an item of a non-public city only when it is 確認可能
				// (datacatalogv3 filterPlateauByPriority), and shows it publicly only when the
				// feature type is public as well
				if st := cmsintegrationcommon.FeatureItemFrom(i).Status; !city.CityPublic && (st == nil || st.Name != string(cmsintegrationcommon.ManagementStatusReady)) {
					continue
				}
				key := city.CityCode + "/" + ft.Code
				if cur, ok := latest[key]; ok && cur.target.Year >= p.Year {
					continue
				}
				latest[key] = reconvertCandidate{
					target: ReconvertTarget{ProjectID: p.ID, Year: p.Year, CityCode: city.CityCode, CityName: city.CityName, Model: model, ItemID: i.ID},
					item:   i,
					city:   city,
				}
			}
		}
	}

	res := make([]ReconvertTarget, 0, len(latest))
	for _, c := range latest {
		t := c.target
		t.Skip = reconvertSkipReason(c, featureTypes, plateaucms.PlateauSpecList(specs))
		res = append(res, t)
	}
	sort.Slice(res, func(i, j int) bool {
		if res[i].CityCode != res[j].CityCode {
			return res[i].CityCode < res[j].CityCode
		}
		return res[i].Model < res[j].Model
	})
	return res, nil
}

func reconvertSkipReason(c reconvertCandidate, featureTypes plateaucms.PlateauFeatureTypeList, specs plateaucms.PlateauSpecList) string {
	fi := featureItemWithAssets(c.item)
	if fi.CityGML == "" {
		return skipNoCityGML
	}
	urls := dataAssetURLs(c.item)
	if slices.ContainsFunc(urls, cmsintegrationcommon.IsJGD2024Asset) {
		return skipAlreadyJGD2024
	}
	// MVT has no heights and is the same in every CRS, so an item with only MVT has nothing to
	// convert, and its conversion never adds a JGD2024 asset
	if !slices.ContainsFunc(urls, func(u string) bool { return !cmsintegrationcommon.IsCRSIndependentAsset(u) }) {
		return skipNoHeights
	}
	specv := fi.SpecMajorVersionInt()
	if specv == 0 {
		specv = c.city.SpecMajorVersionInt()
	}
	if specv < minReconvertSpecMajor {
		return skipOldSpec
	}
	if err := cmsintflow.FlowConversionError(c.target.Model, fi, c.city, featureTypes, specs); err != nil {
		return skipNotFlow + ": " + err.Error()
	}
	if isTag(fi.QCStatus, cmsintegrationcommon.ConvertionStatusRunning) || isTag(fi.ConvertionStatus, cmsintegrationcommon.ConvertionStatusRunning) {
		return skipRunning
	}
	return ""
}

// featureItemWithAssets reads a feature item that was loaded with assets, in which asset
// fields hold asset objects instead of IDs.
func featureItemWithAssets(i *cms.Item) *cmsintegrationcommon.FeatureItem {
	fi := cmsintegrationcommon.FeatureItemFrom(i)
	if a := i.FieldByKey("citygml").GetValue().Asset(); a != nil {
		fi.CityGML = a.ID
		if fi.CityGML == "" {
			fi.CityGML = a.URL
		}
	}
	return fi
}

// dataAssetURLs returns the URLs of the assets in data, including those in the items group.
func dataAssetURLs(i *cms.Item) []string {
	var res []string
	for _, f := range i.Fields {
		if f == nil || f.Key != "data" {
			continue
		}
		values, ok := f.Value.([]any)
		if !ok {
			values = []any{f.Value}
		}
		for _, v := range values {
			if m, ok := v.(map[string]any); ok {
				if u, _ := m["url"].(string); u != "" {
					res = append(res, u)
				}
			}
		}
	}
	return res
}

func isTag(t *cms.Tag, s cmsintegrationcommon.ConvertionStatus) bool {
	return t != nil && t.Name == string(s)
}

type reconverter struct {
	s    *Services
	inp  ReconvertJGD2024Input
	http *http.Client
}

// convert starts the conversion of the target and follows it until it ends.
func (r *reconverter) convert(ctx context.Context, t ReconvertTarget) ReconvertRecord {
	rec := func(kind ReconvertResultKind, format string, args ...any) ReconvertRecord {
		return ReconvertRecord{Time: time.Now(), Target: t, Result: kind, Message: fmt.Sprintf(format, args...)}
	}

	if err := r.start(ctx, t); err != nil {
		return rec(ReconvertFailed, "failed to start: %v", err)
	}

	deadline := time.Now().Add(r.inp.Timeout)
	for {
		select {
		case <-ctx.Done():
			return rec(ReconvertTimeout, "interrupted while the conversion is running")
		case <-time.After(r.inp.PollInterval):
		}

		item, err := r.s.CMS.GetItem(ctx, t.ItemID, true)
		if err != nil {
			if time.Now().After(deadline) {
				return rec(ReconvertTimeout, "failed to get the item: %v", err)
			}
			continue
		}
		status, err := r.convStatus(ctx, item)
		if err != nil {
			if time.Now().After(deadline) {
				return rec(ReconvertTimeout, "failed to get conv_status: %v", err)
			}
			continue
		}

		switch status {
		case string(cmsintegrationcommon.ConvertionStatusSuccess):
			if slices.ContainsFunc(dataAssetURLs(item), cmsintegrationcommon.IsJGD2024Asset) {
				return rec(ReconvertDone, "")
			}
			return rec(ReconvertNoJGD2024, "the conversion succeeded but data has no JGD2024 asset")
		case string(cmsintegrationcommon.ConvertionStatusError):
			return rec(ReconvertFailed, "the conversion ended with an error (see the comments of the item)")
		}

		if time.Now().After(deadline) {
			return rec(ReconvertTimeout, "conv_status is still %q after %s", status, r.inp.Timeout)
		}
	}
}

// convStatus reads conv_status of an item got from the integration API, which has its metadata
// in metadataFields.
func (r *reconverter) convStatus(ctx context.Context, item *cms.Item) (string, error) {
	if len(item.MetadataFields) == 0 && item.MetadataItemID != nil {
		m, err := r.s.CMS.GetItem(ctx, *item.MetadataItemID, false)
		if err != nil {
			return "", err
		}
		item = &cms.Item{MetadataFields: m.Fields}
	}
	if tag := item.MetadataFieldByKey("conv_status").GetValue().Tag(); tag != nil {
		return tag.Name, nil
	}
	return "", nil
}

func (r *reconverter) start(ctx context.Context, t ReconvertTarget) error {
	body, err := json.Marshal(cmsintflow.ReconvertRequest{
		ProjectID: t.ProjectID,
		Model:     t.Model,
		ItemID:    t.ItemID,
		// a result that is not JGD2024 is not merged, so the JGD2011 assets are never replaced
		CRS: string(cmsintegrationcommon.CRSJGD2024),
	})
	if err != nil {
		return err
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, strings.TrimSuffix(r.inp.ServerURL, "/")+"/reconvert", bytes.NewReader(body))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")
	token, err := r.inp.Token.Token()
	if err != nil {
		return fmt.Errorf("failed to get an id token: %w", err)
	}
	req.Header.Set("Authorization", "Bearer "+token.AccessToken)

	res, err := r.http.Do(req)
	if err != nil {
		return err
	}
	defer func() { _ = res.Body.Close() }()
	b, _ := io.ReadAll(io.LimitReader(res.Body, 4096))
	if res.StatusCode != http.StatusOK {
		return fmt.Errorf("%s: %s", res.Status, strings.TrimSpace(string(b)))
	}

	var result cmsintflow.ReconvertResult
	if err := json.Unmarshal(b, &result); err != nil {
		return fmt.Errorf("invalid response: %w", err)
	}
	if !result.Started {
		return fmt.Errorf("not started (conv_status: %q): %s", result.ConvStatus, result.Message)
	}
	return nil
}

// readReconvertState returns the IDs of the items recorded as done.
func readReconvertState(path string) (map[string]bool, error) {
	done := map[string]bool{}
	if path == "" {
		return done, nil
	}
	f, err := os.Open(path)
	if errors.Is(err, os.ErrNotExist) {
		return done, nil
	}
	if err != nil {
		return nil, fmt.Errorf("failed to open the state file: %w", err)
	}
	defer func() { _ = f.Close() }()

	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 0, 64*1024), 1024*1024)
	for sc.Scan() {
		var rec ReconvertRecord
		if err := json.Unmarshal(sc.Bytes(), &rec); err != nil {
			continue
		}
		// a later record of the same item wins
		done[rec.Target.ItemID] = rec.Result == ReconvertDone
	}
	if err := sc.Err(); err != nil {
		return nil, fmt.Errorf("failed to read the state file: %w", err)
	}
	return done, nil
}

type reconvertState struct {
	f *os.File
}

func openReconvertState(path string) (*reconvertState, error) {
	if path == "" {
		return &reconvertState{}, nil
	}
	f, err := os.OpenFile(path, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644)
	if err != nil {
		return nil, fmt.Errorf("failed to open the state file: %w", err)
	}
	return &reconvertState{f: f}, nil
}

func (s *reconvertState) Write(rec ReconvertRecord) error {
	if s.f == nil {
		return nil
	}
	b, err := json.Marshal(rec)
	if err != nil {
		return err
	}
	_, err = s.f.Write(append(b, '\n'))
	return err
}

func (s *reconvertState) Close() error {
	if s.f == nil {
		return nil
	}
	return s.f.Close()
}

func sortedKeys[V any](m map[string]V) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}
