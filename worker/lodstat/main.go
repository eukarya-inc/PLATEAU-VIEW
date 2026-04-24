package lodstat

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"path"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/dustin/go-humanize"
	"github.com/klauspost/compress/flate"
	"github.com/klauspost/compress/zip"
	"github.com/orisano/gosax/xmlb"
	"github.com/reearth/reearthx/log"
	"golang.org/x/sync/errgroup"
	"golang.org/x/sync/semaphore"

	"github.com/eukarya-inc/PLATEAU-VIEW/worker/workerutil"
	cms "github.com/reearth/reearth-cms-api/go"
)

type Config struct {
	SrcURL      string
	Parallelism int
	Feature     string
	CMSURL      string
	CMSToken    string
	ProjectID   string
	ItemID      string
}

func Run(ctx context.Context, cfg Config) error {
	log.Infofc(ctx, "lodstat: starting with config: src=%s, project=%s, item=%s, feature=%s",
		cfg.SrcURL, cfg.ProjectID, cfg.ItemID, cfg.Feature)

	cmsClient, err := cms.New(cfg.CMSURL, cfg.CMSToken)
	if err != nil {
		log.Errorfc(ctx, "lodstat: failed to create CMS client: %v", err)
		return fmt.Errorf("cms: %w", err)
	}

	if err := run(ctx, cmsClient, cfg); err != nil {
		log.Errorfc(ctx, "lodstat: run failed: %v", err)

		if _, updateErr := cmsClient.UpdateItem(ctx, cfg.ItemID, nil, []*cms.Field{
			{
				Key:   "maxlod_status",
				Type:  "tag",
				Value: "エラー",
			},
		}); updateErr != nil {
			log.Errorfc(ctx, "lodstat: failed to update feature status: %v", updateErr)
		}

		if commentErr := cmsClient.CommentToItem(ctx, cfg.ItemID, fmt.Sprintf("最大LOD抽出でエラーが発生しました。%s", err)); commentErr != nil {
			log.Errorfc(ctx, "lodstat: failed to add comment: %v", commentErr)
		}

		// Return the original error so it can be properly handled
		return err
	}

	log.Infofc(ctx, "lodstat: completed successfully")
	return nil
}

func run(ctx context.Context, cmsClient *cms.CMS, cfg Config) error {
	begin := time.Now()
	log.Debugfc(ctx, "lodstat: parsing URL: %s", cfg.SrcURL)

	u, err := url.Parse(cfg.SrcURL)
	if err != nil {
		return fmt.Errorf("failed to parse URL: %w", err)
	}

	log.Infofc(ctx, "lodstat: processing %s file from %s", u.Scheme, u.Host)

	switch u.Scheme {
	case "http", "https":
		log.Debugfc(ctx, "lodstat: opening remote zip file")
		rz, err := newRemoteZip(http.DefaultClient, u.String())
		if err != nil {
			return fmt.Errorf("remote zip: %w", err)
		}
		log.Infofc(ctx, "lodstat: remote zip opened, found %d files", len(rz.File))

		ucGMLSize := uint64(0)
		cGMLSize := uint64(0)
		cTotal := uint64(0)
		gml := uint64(0)

		sem := semaphore.NewWeighted(int64(cfg.Parallelism))
		var eg errgroup.Group

		csvBuf := &bytes.Buffer{}
		w := &maxLodWriter{w: csvBuf}
		if err := w.WriteHeader(); err != nil {
			return fmt.Errorf("write header: %w", err)
		}
		for _, f := range rz.File {
			cTotal += f.CompressedSize64
			normalized := workerutil.NormalizeZipFilePath(f.Name)
			if normalized == "" {
				continue
			}
			if strings.HasSuffix(normalized, ".gml") {
				ucGMLSize += f.UncompressedSize64
				cGMLSize += f.CompressedSize64
				gml++

				name := path.Base(normalized)
				mesh, feature, err := parseCityModelFileEntryName(name)
				if err != nil {
					return fmt.Errorf("parse name: %w", err)
				}
				size := f.UncompressedSize64
				if err := sem.Acquire(ctx, 1); err != nil {
					return fmt.Errorf("acquire semaphore: %w", err)
				}
				rc, err := f.Open(ctx)
				if err != nil {
					return fmt.Errorf("open remote file: %w", err)
				}
				eg.Go(func() error {
					defer func() { _ = rc.Close() }()
					defer sem.Release(1)
					features, lod, err := collectLOD(rc)
					if err != nil {
						return err
					}
					return w.Write(maxLodEntry{
						Mesh:        mesh,
						FeatureType: feature,
						FileName:    name,
						FileSize:    size,
						Features:    features,
						Lod:         lod,
					})
				})
			}
		}
		if err := eg.Wait(); err != nil {
			return err
		}
		d := time.Since(begin).Seconds()
		log.Infofc(ctx, "files=%d gml=%d processed=%s throughput=%s/s downloadRate=%.3f (%s/%s) ", len(rz.File), gml, humanize.Bytes(ucGMLSize), humanize.Bytes(uint64(float64(ucGMLSize)/d)), float64(cGMLSize)/float64(cTotal), humanize.Bytes(cGMLSize), humanize.Bytes(cTotal))

		// Get the base filename without extension
		baseFileName := path.Base(u.Path)
		baseFileName = strings.TrimSuffix(baseFileName, ".zip")

		// Append _lodstat.csv to the base filename
		assetName := fmt.Sprintf("%s_lodstat.csv", baseFileName)
		log.Infofc(ctx, "lodstat: uploading CSV as %s", assetName)

		assetID, err := cmsClient.UploadAssetDirectly(ctx, cfg.ProjectID, assetName, csvBuf)
		if err != nil {
			return fmt.Errorf("upload asset: %w", err)
		}
		log.Infofc(ctx, "lodstat: asset uploaded with ID: %s", assetID)

		log.Debugfc(ctx, "lodstat: updating item %s with maxlod asset", cfg.ItemID)
		_, err = cmsClient.UpdateItem(ctx, cfg.ItemID, []*cms.Field{
			{Key: "maxlod", Value: assetID},
		}, []*cms.Field{
			{Key: "maxlod_status", Type: "tag", Value: "成功"},
		})
		if err != nil {
			return fmt.Errorf("update item: %w", err)
		}
		log.Infofc(ctx, "lodstat: CMS item updated successfully")

		// Add success comment to CMS
		_ = cmsClient.CommentToItem(ctx, cfg.ItemID, "LOD抽出が完了しました。")
	default:
		return fmt.Errorf("unsupported scheme: %s", u.Scheme)
	}
	return nil
}

type remoteZip struct {
	z    *zip.Reader
	rzra *remoteZipReaderAt
	File []remoteZipFile
}

func newRemoteZip(client *http.Client, url string) (*remoteZip, error) {
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	contentLength, ok, err := httpHead(client, req)
	if err != nil {
		return nil, err
	}
	if contentLength == 0 {
		return nil, errors.New("Content-Length is 0")
	}
	if !ok {
		return nil, errors.New("Accept-Ranges is not bytes")
	}
	ra := &httpRangeBytesReaderAt{req: req, client: client}
	rzra := &remoteZipReaderAt{contentLength: contentLength, ra: ra, rar: &bufferedReaderAt{ra: ra}}
	z, err := zip.NewReader(rzra, contentLength)
	if err != nil {
		return nil, err
	}
	rz := &remoteZip{
		z:    z,
		rzra: rzra,
		File: make([]remoteZipFile, 0, len(z.File)),
	}
	for _, f := range z.File {
		rz.File = append(rz.File, remoteZipFile{
			File: f,
			rz:   rz,
		})
	}
	z.File = nil
	return rz, nil
}

type remoteZipFile struct {
	*zip.File
	rz *remoteZip
}

func (r *remoteZipFile) Open(ctx context.Context) (io.ReadCloser, error) {
	dataOffset, err := r.DataOffset()
	if err != nil {
		return nil, err
	}
	begin := dataOffset
	end := begin + int64(r.CompressedSize64)

	rc, err := r.rz.rzra.ra.RangeReader(ctx, begin, end)
	if err != nil {
		return nil, err
	}
	switch r.Method {
	case zip.Deflate:
		return flate.NewReader(rc), nil
	case zip.Store:
		return rc, nil
	default:
		return nil, fmt.Errorf("unsupported method: %d", r.Method)
	}
}

type bufferedReaderAt struct {
	ra io.ReaderAt

	mu   sync.Mutex
	base int64
	buf  []byte
}

func (r *bufferedReaderAt) ReadAt(p []byte, off int64) (n int, err error) {
	r.mu.Lock()
	defer r.mu.Unlock()
	if len(r.buf) == 0 || r.base > off || off+int64(len(p)) > r.base+int64(len(r.buf)) {
		readLen := int64(len(p))
		// optimization for bufio.Reader
		if readLen == 4096 { // bufio default buf size
			// readLen = 64 * 1024
			readLen = 1 * 1024 * 1024
		}
		if int64(len(r.buf)) != readLen {
			r.buf = make([]byte, readLen)
		}
		n, err = r.ra.ReadAt(r.buf, off)
		if err != nil && !errors.Is(err, io.EOF) {
			return 0, err
		}
		r.base = off
		r.buf = r.buf[:n]
	}
	b := r.buf[off-r.base:]
	return copy(p, b), err
}

type httpRangeBytesReaderAt struct {
	req    *http.Request
	client *http.Client
}

func (r *httpRangeBytesReaderAt) ReadAt(p []byte, off int64) (n int, err error) {
	rc, err := r.RangeReader(context.Background(), off, off+int64(len(p)))
	if err != nil {
		return 0, err
	}
	defer func() { _ = rc.Close() }()
	n, err = io.ReadFull(rc, p)
	if err != nil && errors.Is(err, io.ErrUnexpectedEOF) {
		err = nil
	}
	return
}

func (r *httpRangeBytesReaderAt) RangeReader(ctx context.Context, begin, end int64) (io.ReadCloser, error) {
	req := r.req.Clone(ctx)
	v := r.rangeString(begin, end)
	req.Header.Set("Range", v)
	resp, err := r.client.Do(req)
	if err != nil {
		return nil, err
	}
	if resp.StatusCode != http.StatusPartialContent {
		_ = resp.Body.Close()
		return nil, fmt.Errorf("not partial content: %d", resp.StatusCode)
	}
	return resp.Body, nil
}

func (r *httpRangeBytesReaderAt) rangeString(begin, end int64) string {
	var sb strings.Builder
	sb.WriteString("bytes=")
	sb.WriteString(strconv.FormatInt(begin, 10))
	sb.WriteByte('-')
	if end != -1 {
		sb.WriteString(strconv.FormatInt(end-1, 10))
	}
	return sb.String()
}

type remoteZipReaderAt struct {
	contentLength int64

	ra  *httpRangeBytesReaderAt
	rar io.ReaderAt
}

func (r *remoteZipReaderAt) ReadAt(p []byte, off int64) (n int, err error) {
	return r.rar.ReadAt(p, off)
}

func collectLOD(r io.Reader) (int, []int, error) {
	features := 0
	var lod [5]int
	var featureLOD [5]bool
	dec := xmlb.NewDecoder(r, make([]byte, 16*1024))
	for {
		tok, err := dec.Token()
		if err == io.EOF {
			break
		}
		if err != nil {
			return 0, nil, err
		}
		switch tok.Type() {
		case xmlb.StartElement:
			se, err := tok.StartElement()
			if err != nil {
				return 0, nil, err
			}
			if se.Name.Space == "core" && se.Name.Local == "cityObjectMember" {
				features++
				// reset and count
				for i := range featureLOD {
					if featureLOD[i] {
						lod[i]++
					}
					featureLOD[i] = false
				}
			}
			if strings.HasPrefix(se.Name.Local, "lod") && len(se.Name.Local) >= 4 {
				c := se.Name.Local[3]
				if '0' <= c && c <= '4' {
					featureLOD[c-'0'] = true
				}
			}
			if se.Name.Space == "dem" && se.Name.Local == "lod" {
				val, err := dec.Text()
				if err != nil {
					return 0, nil, err
				}
				v, err := strconv.ParseInt(val, 10, 64)
				if err != nil {
					return 0, nil, err
				}
				if 0 <= v && v <= 4 {
					featureLOD[v] = true
				}
			}
		}
	}
	// last element
	for i := range featureLOD {
		if featureLOD[i] {
			lod[i]++
		}
		featureLOD[i] = false
	}
	return features, lod[:], nil
}

// parseCityModelFileEntryName は3D都市モデル標準製品仕様書 7.2.3 に則ってファイル名からメッシュコード, 地物型を取得する
// ref: https://www.mlit.go.jp/plateaudocument/toc7/toc7_02/toc7_02_03/
func parseCityModelFileEntryName(name string) (mesh, feature string, err error) {
	ss := strings.SplitN(name, "_", 3)
	if len(ss) != 3 {
		return "", "", fmt.Errorf("invalid city model file: %s", name)
	}
	return ss[0], ss[1], nil
}

func httpHead(client *http.Client, req *http.Request) (int64, bool, error) {
	req = req.Clone(context.Background())
	req.Method = http.MethodHead
	resp, err := client.Do(req)
	if err != nil {
		return 0, false, err
	}
	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode == http.StatusMethodNotAllowed {
		return 0, false, nil
	}
	return resp.ContentLength, resp.Header.Get("Accept-Ranges") == "bytes", nil
}

type maxLodWriter struct {
	w  io.Writer
	mu sync.Mutex
}

func (w *maxLodWriter) WriteHeader() error {
	_, err := fmt.Fprintln(w.w, "code,type,maxLod,file,filesize,features,lod0,lod1,lod2,lod3,lod4")
	return err
}

func (w *maxLodWriter) Write(e maxLodEntry) error {
	w.mu.Lock()
	defer w.mu.Unlock()
	_, err := fmt.Fprintf(w.w, "%s,%s,%d,%s,%d,%d,%d,%d,%d,%d,%d\n",
		e.Mesh, e.FeatureType, e.maxLod(), e.FileName, e.FileSize, e.Features, e.Lod[0], e.Lod[1], e.Lod[2], e.Lod[3], e.Lod[4])
	return err
}

type maxLodEntry struct {
	Mesh        string
	FeatureType string
	FileName    string
	FileSize    uint64
	Features    int
	Lod         []int
}

func (e *maxLodEntry) maxLod() int {
	x := 0
	for i, v := range e.Lod {
		if v > 0 {
			x = i
		}
	}
	return x
}
