package datacatalog

import (
	"compress/gzip"
	"context"
	"net/http"
	"net/http/httptest"
	"net/url"
	"testing"

	"github.com/jarcoal/httpmock"
	"github.com/reearth/reearthx/rerror"
	"github.com/stretchr/testify/assert"
)

func TestFetchCSV(t *testing.T) {
	tests := []struct {
		name        string
		csvContent  string
		statusCode  int
		prefix      string
		wantRecords [][]string
		wantErr     bool
		wantErrIs   error
	}{
		{
			name: "valid csv with multiple rows",
			csvContent: `12345,bldg,2,path/to/file.gml,1024,100,10,20,30,0,0
67890,tran,1,path/to/file2.gml,2048,200,50,60,0,0,0`,
			statusCode: http.StatusOK,
			prefix:     "https://example.com",
			wantRecords: [][]string{
				{"https://example.com", "12345", "bldg", "2", "path/to/file.gml", "1024", "100", "10", "20", "30", "0", "0"},
				{"https://example.com", "67890", "tran", "1", "path/to/file2.gml", "2048", "200", "50", "60", "0", "0", "0"},
			},
			wantErr: false,
		},
		{
			name:        "empty csv",
			csvContent:  "",
			statusCode:  http.StatusOK,
			prefix:      "https://example.com",
			wantRecords: nil,
			wantErr:     false,
		},
		{
			name:       "404 not found",
			csvContent: "",
			statusCode: http.StatusNotFound,
			prefix:     "https://example.com",
			wantErr:    true,
			wantErrIs:  rerror.ErrNotFound,
		},
		{
			name:       "500 server error",
			csvContent: "",
			statusCode: http.StatusInternalServerError,
			prefix:     "https://example.com",
			wantErr:    true,
		},
		{
			name:       "csv with BOM",
			csvContent: "\xEF\xBB\xBF" + `12345,bldg,2,path/to/file.gml,1024,100,10,20,30,0,0`,
			statusCode: http.StatusOK,
			prefix:     "https://example.com",
			wantRecords: [][]string{
				{"https://example.com", "12345", "bldg", "2", "path/to/file.gml", "1024", "100", "10", "20", "30", "0", "0"},
			},
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				// Check Accept-Encoding header
				assert.Equal(t, "gzip", r.Header.Get("Accept-Encoding"))

				w.WriteHeader(tt.statusCode)
				if tt.statusCode == http.StatusOK {
					_, _ = w.Write([]byte(tt.csvContent))
				}
			}))
			defer server.Close()

			records, err := fetchCSV(context.Background(), server.URL, tt.prefix)

			if tt.wantErr {
				assert.Error(t, err)
				if tt.wantErrIs != nil {
					assert.ErrorIs(t, err, tt.wantErrIs)
				}
			} else {
				assert.NoError(t, err)
				assert.Equal(t, tt.wantRecords, records)
			}
		})
	}
}

func TestFetchCSVWithGzip(t *testing.T) {
	tests := []struct {
		name        string
		csvContent  string
		useGzip     bool
		wantRecords [][]string
		wantErr     bool
	}{
		{
			name: "gzip compressed csv",
			csvContent: `12345,bldg,2,path/to/file.gml,1024,100,10,20,30,0,0
67890,tran,1,path/to/file2.gml,2048,200,50,60,0,0,0`,
			useGzip: true,
			wantRecords: [][]string{
				{"https://example.com", "12345", "bldg", "2", "path/to/file.gml", "1024", "100", "10", "20", "30", "0", "0"},
				{"https://example.com", "67890", "tran", "1", "path/to/file2.gml", "2048", "200", "50", "60", "0", "0", "0"},
			},
			wantErr: false,
		},
		{
			name: "uncompressed csv (backward compatibility)",
			csvContent: `12345,bldg,2,path/to/file.gml,1024,100,10,20,30,0,0
67890,tran,1,path/to/file2.gml,2048,200,50,60,0,0,0`,
			useGzip: false,
			wantRecords: [][]string{
				{"https://example.com", "12345", "bldg", "2", "path/to/file.gml", "1024", "100", "10", "20", "30", "0", "0"},
				{"https://example.com", "67890", "tran", "1", "path/to/file2.gml", "2048", "200", "50", "60", "0", "0", "0"},
			},
			wantErr: false,
		},
		{
			name:        "gzip compressed empty csv",
			csvContent:  "",
			useGzip:     true,
			wantRecords: nil,
			wantErr:     false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				// Verify Accept-Encoding header is set
				assert.Equal(t, "gzip", r.Header.Get("Accept-Encoding"))

				if tt.useGzip {
					w.Header().Set("Content-Encoding", "gzip")
					gzipWriter := gzip.NewWriter(w)
					_, _ = gzipWriter.Write([]byte(tt.csvContent))
					_ = gzipWriter.Close()
				} else {
					_, _ = w.Write([]byte(tt.csvContent))
				}
			}))
			defer server.Close()

			records, err := fetchCSV(context.Background(), server.URL, "https://example.com")

			if tt.wantErr {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
				assert.Equal(t, tt.wantRecords, records)
			}
		})
	}
}

func TestFetchCSVWithInvalidGzip(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Encoding", "gzip")
		// Write invalid gzip data
		_, _ = w.Write([]byte("this is not gzip data"))
	}))
	defer server.Close()

	_, err := fetchCSV(context.Background(), server.URL, "https://example.com")
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "failed to create gzip reader")
}

func TestFetchCSVs(t *testing.T) {
	tests := []struct {
		name            string
		urls            []string
		citygmlBaseURLs []string
		csvContents     []string
		wantErr         bool
		wantRecordsLen  int
	}{
		{
			name:            "multiple valid csvs",
			urls:            []string{"url1", "url2"},
			citygmlBaseURLs: []string{"base1", "base2"},
			csvContents: []string{
				"12345,bldg,2,path/to/file.gml,1024,100,10,20,30,0,0",
				"67890,tran,1,path/to/file2.gml,2048,200,50,60,0,0,0",
			},
			wantErr:        false,
			wantRecordsLen: 2,
		},
		{
			name:            "empty lists",
			urls:            []string{},
			citygmlBaseURLs: []string{},
			csvContents:     []string{},
			wantErr:         false,
			wantRecordsLen:  0,
		},
		{
			name:            "mismatched lengths",
			urls:            []string{"url1", "url2"},
			citygmlBaseURLs: []string{"base1"},
			csvContents:     []string{""},
			wantErr:         true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			httpmock.Activate()
			defer httpmock.DeactivateAndReset()

			for i, csvURL := range tt.urls {
				if i < len(tt.csvContents) {
					httpmock.RegisterResponder("GET", csvURL,
						httpmock.NewStringResponder(http.StatusOK, tt.csvContents[i]))
				}
			}

			records, err := fetchCSVs(context.Background(), tt.urls, tt.citygmlBaseURLs)

			if tt.wantErr {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
				assert.Len(t, records, tt.wantRecordsLen)
			}
		})
	}
}

func TestCsvToCityGMLFilesResponse(t *testing.T) {
	tests := []struct {
		name    string
		data    [][]string
		gmlURLs []*url.URL
		want    CityGMLFiles
	}{
		{
			name: "valid data with all fields",
			data: [][]string{
				{"https://example.com", "53394547", "bldg", "2", "path/to/file.gml", "1024", "100", "10", "20", "30", "0", "0"},
				{"https://example.com", "53394548", "bldg", "1", "path/to/file2.gml", "2048", "150", "15", "25", "0", "0", "0"},
				{"https://example.com", "53394549", "tran", "0", "path/to/file3.gml", "512", "50", "50", "0", "0", "0", "0"},
			},
			gmlURLs: nil,
			want: CityGMLFiles{
				"bldg": {
					{MeshCode: "53394547", MaxLOD: 2, URL: "https://example/udx/bldg/path/to/file.gml", FileSize: int64Ptr(1024), Features: intPtr(100), LOD0: intPtr(10), LOD1: intPtr(20), LOD2: intPtr(30), LOD3: intPtr(0), LOD4: intPtr(0)},
					{MeshCode: "53394548", MaxLOD: 1, URL: "https://example/udx/bldg/path/to/file2.gml", FileSize: int64Ptr(2048), Features: intPtr(150), LOD0: intPtr(15), LOD1: intPtr(25), LOD2: intPtr(0), LOD3: intPtr(0), LOD4: intPtr(0)},
				},
				"tran": {
					{MeshCode: "53394549", MaxLOD: 0, URL: "https://example/udx/tran/path/to/file3.gml", FileSize: int64Ptr(512), Features: intPtr(50), LOD0: intPtr(50), LOD1: intPtr(0), LOD2: intPtr(0), LOD3: intPtr(0), LOD4: intPtr(0)},
				},
			},
		},
		{
			name: "skip header row",
			data: [][]string{
				{"https://example.com", "code", "type", "maxLod", "file", "filesize", "features", "lod0", "lod1", "lod2", "lod3", "lod4"},
				{"https://example.com", "53394547", "bldg", "2", "path/to/file.gml", "1024", "100", "10", "20", "30", "0", "0"},
			},
			gmlURLs: nil,
			want: CityGMLFiles{
				"bldg": {
					{MeshCode: "53394547", MaxLOD: 2, URL: "https://example/udx/bldg/path/to/file.gml", FileSize: int64Ptr(1024), Features: intPtr(100), LOD0: intPtr(10), LOD1: intPtr(20), LOD2: intPtr(30), LOD3: intPtr(0), LOD4: intPtr(0)},
				},
			},
		},
		{
			name: "skip invalid records",
			data: [][]string{
				{"", "53394547", "bldg", "2", "path/to/file.gml", "1024", "100", "10", "20", "30", "0", "0"},            // empty prefix
				{"https://example.com", "", "bldg", "2", "path/to/file.gml", "1024", "100", "10", "20", "30", "0", "0"}, // empty code
			},
			gmlURLs: nil,
			want:    CityGMLFiles{},
		},
		{
			name: "expand short records",
			data: [][]string{
				{"https://example.com", "53394547", "bldg", "2", "path/to/file.gml", "1024", "100"},
			},
			gmlURLs: nil,
			want: CityGMLFiles{
				"bldg": {
					{MeshCode: "53394547", MaxLOD: 2, URL: "https://example/udx/bldg/path/to/file.gml", FileSize: int64Ptr(1024), Features: intPtr(100), LOD0: nil, LOD1: nil, LOD2: nil, LOD3: nil, LOD4: nil},
				},
			},
		},
		{
			name: "match with gml urls",
			data: [][]string{
				{"https://example.com", "53394547", "bldg", "2", "", "1024", "100", "10", "20", "30", "0", "0"},
			},
			gmlURLs: []*url.URL{
				mustParseURL("https://storage.example.com/53394547_bldg_op.gml"),
				mustParseURL("https://storage.example.com/53394548_tran_op.gml"),
			},
			want: CityGMLFiles{
				"bldg": {
					{MeshCode: "53394547", MaxLOD: 2, URL: "https://storage.example.com/53394547_bldg_op.gml", FileSize: int64Ptr(1024), Features: intPtr(100), LOD0: intPtr(10), LOD1: intPtr(20), LOD2: intPtr(30), LOD3: intPtr(0), LOD4: intPtr(0)},
				},
			},
		},
		{
			name: "sorted by mesh code",
			data: [][]string{
				{"https://example.com", "53394549", "bldg", "2", "path/to/file3.gml", "512", "50", "50", "0", "0", "0", "0"},
				{"https://example.com", "53394547", "bldg", "2", "path/to/file.gml", "1024", "100", "10", "20", "30", "0", "0"},
				{"https://example.com", "53394548", "bldg", "1", "path/to/file2.gml", "2048", "150", "15", "25", "0", "0", "0"},
			},
			gmlURLs: nil,
			want: CityGMLFiles{
				"bldg": {
					{MeshCode: "53394547", MaxLOD: 2, URL: "https://example/udx/bldg/path/to/file.gml", FileSize: int64Ptr(1024), Features: intPtr(100), LOD0: intPtr(10), LOD1: intPtr(20), LOD2: intPtr(30), LOD3: intPtr(0), LOD4: intPtr(0)},
					{MeshCode: "53394548", MaxLOD: 1, URL: "https://example/udx/bldg/path/to/file2.gml", FileSize: int64Ptr(2048), Features: intPtr(150), LOD0: intPtr(15), LOD1: intPtr(25), LOD2: intPtr(0), LOD3: intPtr(0), LOD4: intPtr(0)},
					{MeshCode: "53394549", MaxLOD: 2, URL: "https://example/udx/bldg/path/to/file3.gml", FileSize: int64Ptr(512), Features: intPtr(50), LOD0: intPtr(50), LOD1: intPtr(0), LOD2: intPtr(0), LOD3: intPtr(0), LOD4: intPtr(0)},
				},
			},
		},
		{
			name: "handle zero and negative lod counts",
			data: [][]string{
				{"https://example.com", "53394547", "bldg", "2", "path/to/file.gml", "1024", "100", "0", "-1", "10", "0", "0"},
			},
			gmlURLs: nil,
			want: CityGMLFiles{
				"bldg": {
					{MeshCode: "53394547", MaxLOD: 2, URL: "https://example/udx/bldg/path/to/file.gml", FileSize: int64Ptr(1024), Features: intPtr(100), LOD0: intPtr(0), LOD1: intPtr(-1), LOD2: intPtr(10), LOD3: intPtr(0), LOD4: intPtr(0)},
				},
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := csvToCityGMLFilesResponse(tt.data, tt.gmlURLs)
			assert.Equal(t, tt.want, got)
		})
	}
}

// Helper functions
func intPtr(i int) *int {
	return &i
}

func int64Ptr(i int64) *int64 {
	return &i
}

func mustParseURL(s string) *url.URL {
	u, _ := url.Parse(s)
	return u
}
