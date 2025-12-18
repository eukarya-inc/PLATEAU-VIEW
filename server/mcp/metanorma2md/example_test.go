package metanorma2md_test

import (
	"flag"
	"os"
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/mcp/metanorma2md"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/mcp/plateauspecmcp"
)

var exampleFlag = flag.Bool("example", false, "run example tests")

// TestExample_Toc7ToMarkdown demonstrates how to fetch toc7 from PLATEAU spec
// and convert it to markdown.
//
// This test is skipped by default. To run it:
//
//	go test -v -run TestExample_Toc7ToMarkdown ./mcp/metanorma2md -example
//
// Or set the environment variable:
//
//	METANORMA2MD_RUN_EXAMPLES=1 go test -v -run TestExample_Toc7ToMarkdown ./mcp/metanorma2md
func TestExample_Toc7ToMarkdown(t *testing.T) {
	if os.Getenv("METANORMA2MD_RUN_EXAMPLES") == "" && !*exampleFlag {
		t.Skip("Skipping example test. Set METANORMA2MD_RUN_EXAMPLES=1 or use -example flag to run.")
	}

	// Create PLATEAU spec client
	client := plateauspecmcp.NewClient("standard")

	// Fetch toc7 with all children
	doc, err := client.GetContentWithChildren("/plateaudocument/toc7")
	if err != nil {
		t.Fatalf("Failed to fetch toc7: %v", err)
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

	// Write to file
	outputPath := "testdata/toc7_example.md"
	if err := os.MkdirAll("testdata", 0755); err != nil {
		t.Fatalf("Failed to create testdata dir: %v", err)
	}
	if err := os.WriteFile(outputPath, []byte(markdown), 0644); err != nil {
		t.Fatalf("Failed to write file: %v", err)
	}

	t.Logf("Successfully wrote %d bytes to %s", len(markdown), outputPath)
}
