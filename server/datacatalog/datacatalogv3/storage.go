package datacatalogv3

import (
	"compress/gzip"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path"
	"path/filepath"
	"strings"
	"time"

	"cloud.google.com/go/storage"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/reearth/reearthx/log"
	"google.golang.org/api/iterator"
)

// CacheStorage provides read/write access for datacatalog cache
type CacheStorage interface {
	RepoWriter
	RepoReader
}

// RepoReader is the interface for reading repository data from cache
type RepoReader interface {
	// List returns all available project names
	List(ctx context.Context) ([]string, error)
	// Load loads InMemoryRepoContext for a project
	Load(ctx context.Context, project string) (*plateauapi.InMemoryRepoContext, error)
	// Close releases any resources held by the reader
	Close() error
}

// NewRepoReaderFromURL creates a RepoReader from a URL.
// Supports gs:// for GCS and local filesystem paths.
func NewRepoReaderFromURL(ctx context.Context, url string) (RepoReader, error) {
	if strings.HasPrefix(url, "gs://") {
		return NewGCSStorage(ctx, url)
	}
	return NewFileRepoReader(url), nil
}

// GCSStorage implements CacheStorage for Google Cloud Storage
type GCSStorage struct {
	client *storage.Client
	bucket string
	prefix string
}

// GCSStorageConfig is the configuration for GCSStorage
type GCSStorageConfig struct {
	// BucketURL is the GCS URL (gs://bucket/prefix)
	BucketURL string
}

// ParseGCSURL parses a GCS URL (gs://bucket/prefix) and returns bucket and prefix
func ParseGCSURL(url string) (bucket, prefix string, err error) {
	if !strings.HasPrefix(url, "gs://") {
		return "", "", fmt.Errorf("invalid GCS URL: must start with gs://")
	}

	url = strings.TrimPrefix(url, "gs://")
	parts := strings.SplitN(url, "/", 2)
	bucket = parts[0]
	if len(parts) > 1 {
		prefix = parts[1]
	}
	return bucket, prefix, nil
}

// NewGCSStorage creates a new GCSStorage
func NewGCSStorage(ctx context.Context, bucketURL string) (*GCSStorage, error) {
	bucket, prefix, err := ParseGCSURL(bucketURL)
	if err != nil {
		return nil, err
	}

	client, err := storage.NewClient(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to create GCS client: %w", err)
	}

	return &GCSStorage{
		client: client,
		bucket: bucket,
		prefix: prefix,
	}, nil
}

// NewGCSStorageWithClient creates a new GCSStorage with an existing client
func NewGCSStorageWithClient(client *storage.Client, bucket, prefix string) *GCSStorage {
	return &GCSStorage{
		client: client,
		bucket: bucket,
		prefix: prefix,
	}
}

func (s *GCSStorage) objectName(project string) string {
	// Cache files are stored in project subdirectories: {prefix}/{project}/repo_{project}.json
	return path.Join(s.prefix, project, fmt.Sprintf("repo_%s.json", project))
}

func (s *GCSStorage) warningObjectName(project string) string {
	// Warning files are stored in project subdirectories: {prefix}/{project}/repo_{project}_warnings.txt
	return path.Join(s.prefix, project, fmt.Sprintf("repo_%s_warnings.txt", project))
}

// GetWriter implements RepoWriter (returns gzip-compressed writer)
func (s *GCSStorage) GetWriter(project string) (io.WriteCloser, error) {
	ctx := context.Background()
	obj := s.client.Bucket(s.bucket).Object(s.objectName(project))
	w := obj.NewWriter(ctx)
	w.ContentType = "application/json"
	w.ContentEncoding = "gzip"
	gw := gzip.NewWriter(w)
	return &gzipWriteCloser{gw: gw, underlying: w}, nil
}

// GetWarningWriter implements RepoWriter (returns gzip-compressed writer)
func (s *GCSStorage) GetWarningWriter(project string) (io.WriteCloser, error) {
	ctx := context.Background()
	obj := s.client.Bucket(s.bucket).Object(s.warningObjectName(project))
	w := obj.NewWriter(ctx)
	w.ContentType = "text/plain"
	w.ContentEncoding = "gzip"
	gw := gzip.NewWriter(w)
	return &gzipWriteCloser{gw: gw, underlying: w}, nil
}

// gzipWriteCloser wraps gzip.Writer to close both gzip and underlying writer
type gzipWriteCloser struct {
	gw         *gzip.Writer
	underlying io.WriteCloser
}

func (w *gzipWriteCloser) Write(p []byte) (n int, err error) {
	return w.gw.Write(p)
}

func (w *gzipWriteCloser) Close() error {
	if err := w.gw.Close(); err != nil {
		_ = w.underlying.Close()
		return err
	}
	return w.underlying.Close()
}

// List implements RepoReader
func (s *GCSStorage) List(ctx context.Context) ([]string, error) {
	query := &storage.Query{
		Prefix: s.prefix,
	}

	var projects []string
	it := s.client.Bucket(s.bucket).Objects(ctx, query)
	for {
		attrs, err := it.Next()
		if err == iterator.Done {
			break
		}
		if err != nil {
			return nil, fmt.Errorf("failed to list objects: %w", err)
		}

		// Extract project name from object path
		// Expected format: {prefix}/{project}/repo_{project}.json
		name := strings.TrimPrefix(attrs.Name, s.prefix)
		name = strings.TrimPrefix(name, "/")

		// Split path to get directory and filename
		parts := strings.Split(name, "/")
		if len(parts) != 2 {
			continue
		}

		dirName := parts[0]
		fileName := parts[1]

		// Check if filename matches repo_{project}.json pattern (not warnings)
		if strings.HasPrefix(fileName, "repo_") && strings.HasSuffix(fileName, ".json") && !strings.Contains(fileName, "_warnings") {
			project := strings.TrimPrefix(fileName, "repo_")
			project = strings.TrimSuffix(project, ".json")
			// Verify directory name matches project name
			if dirName == project {
				projects = append(projects, project)
			}
		}
	}

	return projects, nil
}

// Load implements RepoReader (reads gzip-compressed data)
func (s *GCSStorage) Load(ctx context.Context, project string) (*plateauapi.InMemoryRepoContext, error) {
	obj := s.client.Bucket(s.bucket).Object(s.objectName(project))
	// ReadCompressed(true) prevents GCS from auto-decompressing, keeping transfer size small
	r, err := obj.ReadCompressed(true).NewReader(ctx)
	if err != nil {
		if err == storage.ErrObjectNotExist {
			return nil, fmt.Errorf("cache not found for project %s: %w", project, err)
		}
		return nil, fmt.Errorf("failed to open cache for project %s: %w", project, err)
	}
	defer func() {
		_ = r.Close()
	}()

	// Decompress gzip locally
	gr, err := gzip.NewReader(r)
	if err != nil {
		return nil, fmt.Errorf("failed to create gzip reader for project %s: %w", project, err)
	}
	defer func() {
		_ = gr.Close()
	}()

	var repoCtx plateauapi.InMemoryRepoContext
	if err := json.NewDecoder(gr).Decode(&repoCtx); err != nil {
		return nil, fmt.Errorf("failed to decode cache for project %s: %w", project, err)
	}

	return &repoCtx, nil
}

// Close closes the GCS client
func (s *GCSStorage) Close() error {
	return s.client.Close()
}

// GetCacheTimestamp returns the last updated time of the cache object for a project.
// Returns zero time if the object doesn't exist.
func (s *GCSStorage) GetCacheTimestamp(ctx context.Context, project string) (time.Time, error) {
	obj := s.client.Bucket(s.bucket).Object(s.objectName(project))
	attrs, err := obj.Attrs(ctx)
	if err != nil {
		if err == storage.ErrObjectNotExist {
			return time.Time{}, nil
		}
		return time.Time{}, fmt.Errorf("failed to get object attrs: %w", err)
	}
	return attrs.Updated, nil
}

// FileRepoReader reads from local filesystem
type FileRepoReader struct {
	basedir string
}

// NewFileRepoReader creates a new FileRepoReader
func NewFileRepoReader(basedir string) *FileRepoReader {
	return &FileRepoReader{basedir: basedir}
}

// Close implements RepoReader (no-op for local filesystem)
func (r *FileRepoReader) Close() error {
	return nil
}

// List implements RepoReader
func (r *FileRepoReader) List(ctx context.Context) ([]string, error) {
	entries, err := os.ReadDir(r.basedir)
	if err != nil {
		return nil, fmt.Errorf("failed to read cache directory: %w", err)
	}

	var projects []string
	for _, entry := range entries {
		name := entry.Name()
		// Support both .json and .json.gz
		if strings.HasPrefix(name, "repo_") && !strings.Contains(name, "_warnings") {
			project := strings.TrimPrefix(name, "repo_")
			project = strings.TrimSuffix(project, ".json.gz")
			project = strings.TrimSuffix(project, ".json")
			projects = append(projects, project)
		}
	}

	return projects, nil
}

// Load implements RepoReader
func (r *FileRepoReader) Load(ctx context.Context, project string) (*plateauapi.InMemoryRepoContext, error) {
	// Try .json.gz first, then .json
	gzPath := filepath.Join(r.basedir, fmt.Sprintf("repo_%s.json.gz", project))
	jsonPath := filepath.Join(r.basedir, fmt.Sprintf("repo_%s.json", project))

	var reader io.Reader
	var closer io.Closer

	if f, err := os.Open(gzPath); err == nil {
		closer = f
		gr, err := gzip.NewReader(f)
		if err != nil {
			_ = f.Close()
			return nil, fmt.Errorf("failed to create gzip reader: %w", err)
		}
		reader = gr
		defer func() {
			_ = gr.Close()
		}()
	} else if f, err := os.Open(jsonPath); err == nil {
		closer = f
		reader = f
	} else {
		return nil, fmt.Errorf("cache not found for project %s", project)
	}
	defer func() {
		_ = closer.Close()
	}()

	var repoCtx plateauapi.InMemoryRepoContext
	if err := json.NewDecoder(reader).Decode(&repoCtx); err != nil {
		return nil, fmt.Errorf("failed to decode cache for project %s: %w", project, err)
	}

	return &repoCtx, nil
}

// LoadAllFromStorage loads all projects from a RepoReader into Repos
func (r *Repos) LoadAllFromStorage(ctx context.Context, reader RepoReader) error {
	projects, err := reader.List(ctx)
	if err != nil {
		return fmt.Errorf("failed to list projects from storage: %w", err)
	}

	log.Infofc(ctx, "datacatalogv3: loading %d projects from cache", len(projects))

	for _, project := range projects {
		if err := r.LoadFromStorage(ctx, project, reader); err != nil {
			log.Warnfc(ctx, "datacatalogv3: failed to load project %s from cache: %v", project, err)
			continue
		}
		log.Infofc(ctx, "datacatalogv3: loaded project %s from cache", project)
	}

	return nil
}

// LoadFromStorage loads a single project from storage
func (r *Repos) LoadFromStorage(ctx context.Context, project string, reader RepoReader) error {
	repoCtx, err := reader.Load(ctx, project)
	if err != nil {
		return err
	}

	repo := plateauapi.NewInMemoryRepo(repoCtx)
	r.SetRepo(project, repo, nil)

	return nil
}
