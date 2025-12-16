package datacatalogv3

import (
	"compress/gzip"
	"context"
	"os"
	"path/filepath"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestFileRepoReader_List(t *testing.T) {
	// Create temp directory with test files
	tmpDir := t.TempDir()

	// Create test cache files
	createTestCacheFile(t, tmpDir, "test-project-1")
	createTestCacheFile(t, tmpDir, "test-project-2")
	// Create warning file (should be ignored)
	_ = os.WriteFile(filepath.Join(tmpDir, "repo_test-project-1_warnings.txt"), []byte("warning"), 0644)

	reader := NewFileRepoReader(tmpDir)
	ctx := context.Background()

	projects, err := reader.List(ctx)
	require.NoError(t, err)
	assert.Len(t, projects, 2)
	assert.Contains(t, projects, "test-project-1")
	assert.Contains(t, projects, "test-project-2")
}

func TestFileRepoReader_Load_JSON(t *testing.T) {
	tmpDir := t.TempDir()
	createTestCacheFile(t, tmpDir, "test-project")

	reader := NewFileRepoReader(tmpDir)
	ctx := context.Background()

	repoCtx, err := reader.Load(ctx, "test-project")
	require.NoError(t, err)
	require.NotNil(t, repoCtx)

	assert.Equal(t, "test-project", repoCtx.Name)
	assert.Len(t, repoCtx.Areas.All(), 1)
	assert.Len(t, repoCtx.Datasets.All(), 1)
	assert.Len(t, repoCtx.DatasetTypes.All(), 1)
}

func TestFileRepoReader_Load_GzipJSON(t *testing.T) {
	tmpDir := t.TempDir()
	createTestCacheFileGzip(t, tmpDir, "test-project-gz")

	reader := NewFileRepoReader(tmpDir)
	ctx := context.Background()

	repoCtx, err := reader.Load(ctx, "test-project-gz")
	require.NoError(t, err)
	require.NotNil(t, repoCtx)

	assert.Equal(t, "test-project-gz", repoCtx.Name)
}

func TestFileRepoReader_Load_NotFound(t *testing.T) {
	tmpDir := t.TempDir()

	reader := NewFileRepoReader(tmpDir)
	ctx := context.Background()

	_, err := reader.Load(ctx, "nonexistent")
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "cache not found")
}

// createTestCacheFile creates a minimal test cache JSON file
func createTestCacheFile(t *testing.T, dir, project string) {
	t.Helper()

	jsonData := testCacheJSON(project)
	path := filepath.Join(dir, "repo_"+project+".json")
	require.NoError(t, os.WriteFile(path, []byte(jsonData), 0644))
}

// createTestCacheFileGzip creates a minimal test cache gzip JSON file
func createTestCacheFileGzip(t *testing.T, dir, project string) {
	t.Helper()

	jsonData := testCacheJSON(project)
	path := filepath.Join(dir, "repo_"+project+".json.gz")

	f, err := os.Create(path)
	require.NoError(t, err)
	defer func() {
		_ = f.Close()
	}()

	gw := gzip.NewWriter(f)
	_, err = gw.Write([]byte(jsonData))
	require.NoError(t, err)
	require.NoError(t, gw.Close())
}

func testCacheJSON(name string) string {
	return `{
		"name": "` + name + `",
		"areas": {
			"PREFECTURE": [
				{
					"id": "p_01",
					"type": "PREFECTURE",
					"code": "01",
					"name": "Test Prefecture"
				}
			]
		},
		"datasets": {
			"PLATEAU": [
				{
					"id": "d_test",
					"name": "Test Dataset"
				}
			]
		},
		"datasetTypes": {
			"PLATEAU": [
				{
					"id": "dt_bldg",
					"code": "bldg",
					"name": "建築物モデル"
				}
			]
		},
		"plateauSpecs": [],
		"years": [2024],
		"cityGML": {}
	}`
}
