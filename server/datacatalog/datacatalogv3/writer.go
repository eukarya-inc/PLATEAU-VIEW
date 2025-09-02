package datacatalogv3

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sync"
)

// RepoWriter is a minimal interface for writing repository data
type RepoWriter interface {
	GetWriter(project string) (io.WriteCloser, error)
	GetWarningWriter(project string) (io.WriteCloser, error)
}

// FileRepoWriter writes to filesystem
type FileRepoWriter struct {
	basedir string
}

func NewFileRepoWriter(basedir string) *FileRepoWriter {
	return &FileRepoWriter{basedir: basedir}
}

func (w *FileRepoWriter) GetWriter(project string) (io.WriteCloser, error) {
	// Ensure directory exists
	if err := os.MkdirAll(w.basedir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create directory: %w", err)
	}
	
	path := filepath.Join(w.basedir, fmt.Sprintf("repo_%s.json", project))
	return os.Create(path)
}

func (w *FileRepoWriter) GetWarningWriter(project string) (io.WriteCloser, error) {
	// Ensure directory exists
	if err := os.MkdirAll(w.basedir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create directory: %w", err)
	}
	
	path := filepath.Join(w.basedir, fmt.Sprintf("repo_%s_warnings.txt", project))
	return os.Create(path)
}

// MemRepoWriter writes to memory
type MemRepoWriter struct {
	mu       sync.RWMutex
	data     map[string]*bytes.Buffer
	warnings map[string]*bytes.Buffer
}

func NewMemRepoWriter() *MemRepoWriter {
	return &MemRepoWriter{
		data:     make(map[string]*bytes.Buffer),
		warnings: make(map[string]*bytes.Buffer),
	}
}

func (w *MemRepoWriter) GetWriter(project string) (io.WriteCloser, error) {
	w.mu.Lock()
	defer w.mu.Unlock()
	
	buf := new(bytes.Buffer)
	w.data[project] = buf
	return &nopCloser{Writer: buf}, nil
}

func (w *MemRepoWriter) GetWarningWriter(project string) (io.WriteCloser, error) {
	w.mu.Lock()
	defer w.mu.Unlock()
	
	buf := new(bytes.Buffer)
	w.warnings[project] = buf
	return &nopCloser{Writer: buf}, nil
}

// GetData returns the data for a project
func (w *MemRepoWriter) GetData(project string) ([]byte, bool) {
	w.mu.RLock()
	defer w.mu.RUnlock()
	
	buf, ok := w.data[project]
	if !ok {
		return nil, false
	}
	return buf.Bytes(), true
}

// GetWarnings returns the warnings for a project
func (w *MemRepoWriter) GetWarnings(project string) ([]byte, bool) {
	w.mu.RLock()
	defer w.mu.RUnlock()
	
	buf, ok := w.warnings[project]
	if !ok {
		return nil, false
	}
	return buf.Bytes(), true
}

// nopCloser wraps an io.Writer to add a no-op Close method
type nopCloser struct {
	io.Writer
}

func (nopCloser) Close() error { return nil }