package metanorma2md_test

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/mcp/metanorma2md"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/mcp/plateauspecmcp"
)

var exampleFlag = flag.Bool("example", false, "run example tests")

// TestExample_AllTocsToMarkdown fetches all toc chapters from PLATEAU spec
// and converts each to a separate markdown file.
//
// This test is skipped by default. To run it:
//
//	go test -v -run TestExample_AllTocsToMarkdown ./mcp/metanorma2md -example
//
// Or set the environment variable:
//
//	METANORMA2MD_RUN_EXAMPLES=1 go test -v -run TestExample_AllTocsToMarkdown ./mcp/metanorma2md
func TestExample_AllTocsToMarkdown(t *testing.T) {
	if os.Getenv("METANORMA2MD_RUN_EXAMPLES") == "" && !*exampleFlag {
		t.Skip("Skipping example test. Set METANORMA2MD_RUN_EXAMPLES=1 or use -example flag to run.")
	}

	// Create output directory
	outputDir := "testdata/spec"
	if err := os.MkdirAll(outputDir, 0755); err != nil {
		t.Fatalf("Failed to create output dir: %v", err)
	}

	// Create PLATEAU spec client
	client := plateauspecmcp.NewClient("standard")

	// Get outline to find all toc chapters
	outline, err := client.GetOutlineWithDepth(1)
	if err != nil {
		t.Fatalf("Failed to fetch outline: %v", err)
	}

	t.Logf("Found %d chapters", len(outline))

	// Process each toc chapter
	for _, chapter := range outline {
		t.Run(chapter.ID, func(t *testing.T) {
			// Extract toc ID from path (e.g., "/plateaudocument/toc1" -> "toc1")
			tocID := chapter.ID
			if tocID == "" {
				parts := strings.Split(chapter.Path, "/")
				if len(parts) > 0 {
					tocID = parts[len(parts)-1]
				}
			}

			t.Logf("Processing %s: %s", tocID, chapter.Title)

			// Fetch content with all children
			doc, err := client.GetContentWithChildren(chapter.Path)
			if err != nil {
				t.Errorf("Failed to fetch %s: %v", tocID, err)
				return
			}

			// Convert to metanorma2md.Document
			mdDoc := &metanorma2md.Document{
				Title:    doc.Title,
				Path:     doc.Path,
				Metadata: doc.Metadata,
				Content:  make([]metanorma2md.Content, len(doc.Content)),
			}
			for i, c := range doc.Content {
				mdDoc.Content[i] = metanorma2md.Content{
					Type:    c.Type,
					Content: c.Content,
				}
			}

			// Convert to markdown (without base64 images)
			markdown := metanorma2md.Convert(mdDoc, &metanorma2md.Options{
				IncludeImages: false,
			})

			// Write JSON
			jsonPath := filepath.Join(outputDir, fmt.Sprintf("%s.json", tocID))
			jsonData, err := json.MarshalIndent(doc, "", "  ")
			if err != nil {
				t.Errorf("Failed to marshal JSON for %s: %v", tocID, err)
				return
			}
			if err := os.WriteFile(jsonPath, jsonData, 0644); err != nil {
				t.Errorf("Failed to write JSON for %s: %v", tocID, err)
				return
			}

			// Write Markdown
			mdPath := filepath.Join(outputDir, fmt.Sprintf("%s.md", tocID))
			if err := os.WriteFile(mdPath, []byte(markdown), 0644); err != nil {
				t.Errorf("Failed to write markdown for %s: %v", tocID, err)
				return
			}

			t.Logf("Wrote %s (%d bytes JSON, %d bytes MD)", tocID, len(jsonData), len(markdown))
		})
	}
}
