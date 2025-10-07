package mcp

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/mark3labs/mcp-go/mcp"
)

// HandleResourceList returns a list of available PLATEAU specification resources
func HandleResourceList(ctx context.Context) ([]mcp.Resource, error) {
	resources := []mcp.Resource{
		{
			URI:         "plateau://standard/overview",
			Name:        "PLATEAU Standard Product Specification Overview",
			Description: "Overview and introduction to PLATEAU 3D city model standard specifications",
			MIMEType:    "text/markdown",
		},
		{
			URI:         "plateau://standard/building",
			Name:        "Building Model Specification",
			Description: "Detailed specifications for building models (LOD1-LOD4)",
			MIMEType:    "text/markdown",
		},
		{
			URI:         "plateau://standard/transportation",
			Name:        "Transportation Model Specification",
			Description: "Specifications for roads, railways, and transportation infrastructure",
			MIMEType:    "text/markdown",
		},
		{
			URI:         "plateau://standard/landuse",
			Name:        "Land Use Model Specification",
			Description: "Urban planning and land use classification specifications",
			MIMEType:    "text/markdown",
		},
		{
			URI:         "plateau://standard/disaster",
			Name:        "Disaster Prevention Model Specification",
			Description: "Flood, tsunami, and disaster risk assessment model specifications",
			MIMEType:    "text/markdown",
		},
		{
			URI:         "plateau://procedure/overview",
			Name:        "Standard Work Procedures Overview",
			Description: "Overview of 3D city model creation procedures and workflows",
			MIMEType:    "text/markdown",
		},
		{
			URI:         "plateau://procedure/data-preparation",
			Name:        "Data Preparation Procedures",
			Description: "Procedures for preparing source data for 3D city model creation",
			MIMEType:    "text/markdown",
		},
		{
			URI:         "plateau://procedure/quality-control",
			Name:        "Quality Control Procedures",
			Description: "Quality assurance and validation procedures for 3D city models",
			MIMEType:    "text/markdown",
		},
		{
			URI:         "plateau://glossary",
			Name:        "PLATEAU Glossary",
			Description: "Terminology and definitions used in PLATEAU specifications",
			MIMEType:    "text/markdown",
		},
	}

	return resources, nil
}

// HandleResourceRead reads the content of a specific resource
func HandleResourceRead(ctx context.Context, request mcp.ReadResourceRequest) ([]mcp.ResourceContents, error) {
	uri := request.Params.URI
	// Parse URI
	if !strings.HasPrefix(uri, "plateau://") {
		return nil, fmt.Errorf("invalid resource URI: %s", uri)
	}

	parts := strings.Split(strings.TrimPrefix(uri, "plateau://"), "/")
	if len(parts) < 1 {
		return nil, fmt.Errorf("invalid resource URI format: %s", uri)
	}

	docType := "standard"
	category := parts[0]
	topic := ""
	
	if category == "procedure" && len(parts) > 1 {
		docType = "procedure"
		topic = parts[1]
	} else if category == "standard" && len(parts) > 1 {
		topic = parts[1]
	} else if category == "glossary" {
		content, err := getGlossaryContent()
		if err != nil {
			return nil, err
		}
		return []mcp.ResourceContents{
			mcp.TextResourceContents{
				URI:      uri,
				MIMEType: "text/markdown",
				Text:     content,
			},
		}, nil
	} else {
		topic = category
	}

	// Map topics to actual document paths
	pathMap := map[string]string{
		"overview":        "",
		"building":        "/toc4/toc4_02",
		"transportation":  "/toc4/toc4_03",
		"landuse":        "/toc4/toc4_01",
		"disaster":       "/toc4/toc4_05",
		"data-preparation": "/toc_03",
		"quality-control":  "/toc_06",
	}

	path, ok := pathMap[topic]
	if !ok {
		return nil, fmt.Errorf("unknown resource topic: %s", topic)
	}

	client := NewClient(docType)

	// For overview, get chapter listing
	if path == "" {
		chapters, err := client.ListChapters()
		if err != nil {
			return nil, fmt.Errorf("failed to fetch overview: %w", err)
		}
		content := formatOverview(chapters, docType)
		return []mcp.ResourceContents{
			mcp.TextResourceContents{
				URI:      uri,
				MIMEType: "text/markdown",
				Text:     content,
			},
		}, nil
	}

	// Get sections for the specified path
	sections, err := client.ListSectionsByPath(path)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch sections: %w", err)
	}

	// Get content for first few sections
	var content strings.Builder
	content.WriteString(fmt.Sprintf("# %s\n\n", getTopicTitle(topic)))
	
	maxSections := 3 // Limit to first 3 sections for summary
	for i, section := range sections {
		if i >= maxSections {
			break
		}
		
		doc, err := client.GetContentByPath(section.Path)
		if err != nil {
			continue
		}
		
		content.WriteString(fmt.Sprintf("\n## %s\n\n", section.Title))
		content.WriteString(formatAsMarkdown(doc))
		content.WriteString("\n---\n")
	}

	if len(sections) > maxSections {
		content.WriteString(fmt.Sprintf("\n*Note: Showing first %d sections. Total sections available: %d*\n", maxSections, len(sections)))
	}

	return []mcp.ResourceContents{
		mcp.TextResourceContents{
			URI:      uri,
			MIMEType: "text/markdown",
			Text:     content.String(),
		},
	}, nil
}

func formatOverview(chapters []Chapter, docType string) string {
	var sb strings.Builder
	
	title := "PLATEAU Standard Product Specification"
	if docType == "procedure" {
		title = "3D City Model Standard Work Procedures"
	}
	
	sb.WriteString(fmt.Sprintf("# %s Overview\n\n", title))
	sb.WriteString("## Document Structure\n\n")
	
	for _, chapter := range chapters {
		sb.WriteString(fmt.Sprintf("### %s\n", chapter.Title))
		sb.WriteString(fmt.Sprintf("- **ID**: %s\n", chapter.ID))
		sb.WriteString(fmt.Sprintf("- **Path**: `%s`\n", chapter.Path))
		sb.WriteString("\n")
		
		// Add brief description based on chapter ID
		desc := getChapterDescription(chapter.ID)
		if desc != "" {
			sb.WriteString(desc + "\n\n")
		}
	}
	
	sb.WriteString("\n## How to Navigate\n\n")
	sb.WriteString("Use the following tools to explore the specification:\n\n")
	sb.WriteString("- `plateau_spec_list`: Browse chapters and sections\n")
	sb.WriteString("- `plateau_spec_get_content`: Read specific section content\n")
	sb.WriteString("- `plateau_spec_search`: Search for specific topics\n")
	
	return sb.String()
}

func getGlossaryContent() (string, error) {
	// This would typically fetch from a glossary endpoint
	// For now, return a static glossary of common terms
	glossary := map[string]string{
		"LOD": "Level of Detail - Describes the granularity of 3D model representation",
		"CityGML": "Open data model and XML-based format for storage and exchange of 3D city models",
		"建築物モデル": "Building Model - 3D representation of buildings and structures",
		"道路モデル": "Road Model - 3D representation of transportation infrastructure",
		"土地利用モデル": "Land Use Model - Classification and representation of urban land use",
		"災害リスク": "Disaster Risk - Assessment of natural disaster probabilities and impacts",
		"洪水浸水想定": "Flood Inundation Assumption - Predicted flood depths and areas",
		"都市計画決定情報": "Urban Planning Decision Information - Official urban planning designations",
	}
	
	var sb strings.Builder
	sb.WriteString("# PLATEAU Glossary\n\n")
	
	for term, definition := range glossary {
		sb.WriteString(fmt.Sprintf("## %s\n\n%s\n\n", term, definition))
	}
	
	data, _ := json.MarshalIndent(glossary, "", "  ")
	sb.WriteString("\n## Full Glossary (JSON)\n\n```json\n")
	sb.WriteString(string(data))
	sb.WriteString("\n```\n")
	
	return sb.String(), nil
}

func getTopicTitle(topic string) string {
	titles := map[string]string{
		"building":         "Building Model Specification",
		"transportation":   "Transportation Model Specification",
		"landuse":         "Land Use Model Specification",
		"disaster":        "Disaster Prevention Model Specification",
		"data-preparation": "Data Preparation Procedures",
		"quality-control":  "Quality Control Procedures",
	}
	
	if title, ok := titles[topic]; ok {
		return title
	}
	return "PLATEAU Specification"
}

func getChapterDescription(chapterID string) string {
	descriptions := map[string]string{
		"toc1": "Introduction and overview of PLATEAU specifications",
		"toc2": "Basic concepts and terminology definitions",
		"toc3": "Data product specifications and requirements",
		"toc4": "Detailed model specifications for various urban features",
		"toc5": "Quality requirements and validation procedures",
		"toc6": "Metadata specifications and documentation standards",
		"toc_01": "Overview of standard work procedures",
		"toc_02": "Planning and preparation phase procedures",
		"toc_03": "Data collection and preparation procedures",
		"toc_04": "3D modeling and conversion procedures",
		"toc_05": "Quality assurance and validation procedures",
		"toc_06": "Delivery and documentation procedures",
	}
	
	if desc, ok := descriptions[chapterID]; ok {
		return desc
	}
	return ""
}