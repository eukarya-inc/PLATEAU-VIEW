package plateauspecmcp

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/mark3labs/mcp-go/mcp"
	"github.com/mark3labs/mcp-go/server"
)

// RegisterResources registers PLATEAU specification resources
func RegisterResources(s *server.MCPServer) {
	// Get initial resources
	resources, _ := HandleResourceList(context.Background())

	// Add each resource with its handler
	for _, resource := range resources {
		s.AddResource(resource, HandleResourceRead)
	}
}

// HandleResourceList returns a list of available PLATEAU specification resources
func HandleResourceList(_ context.Context) ([]mcp.Resource, error) {
	resources := []mcp.Resource{
		{
			URI:         "plateau://standard/outline",
			Name:        "PLATEAU Standard Product Specification - Table of Contents",
			Description: "Complete table of contents of the 3D City Model Standard Product Specification",
			MIMEType:    "text/markdown",
		},
		{
			URI:         "plateau://procedure/outline",
			Name:        "PLATEAU Standard Work Procedures - Table of Contents",
			Description: "Complete table of contents of the 3D City Model Standard Work Procedures",
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
func HandleResourceRead(_ context.Context, request mcp.ReadResourceRequest) ([]mcp.ResourceContents, error) {
	uri := request.Params.URI
	// Parse URI
	if !strings.HasPrefix(uri, "plateau://") {
		return nil, fmt.Errorf("invalid resource URI: %s", uri)
	}

	parts := strings.Split(strings.TrimPrefix(uri, "plateau://"), "/")
	if len(parts) < 1 {
		return nil, fmt.Errorf("invalid resource URI format: %s", uri)
	}

	category := parts[0]

	// Handle glossary
	if category == "glossary" {
		content := getGlossaryContent()
		return []mcp.ResourceContents{
			mcp.TextResourceContents{
				URI:      uri,
				MIMEType: "text/markdown",
				Text:     content,
			},
		}, nil
	}

	// Handle outline requests
	if len(parts) >= 2 && parts[1] == "outline" {
		docType := "standard"
		if category == "procedure" {
			docType = "procedure"
		}

		client := NewClient(docType)
		outline, err := client.GetOutlineWithDepth(2)
		if err != nil {
			return nil, fmt.Errorf("failed to get outline: %w", err)
		}

		content := formatOutlineAsMarkdown(outline, docType)
		return []mcp.ResourceContents{
			mcp.TextResourceContents{
				URI:      uri,
				MIMEType: "text/markdown",
				Text:     content,
			},
		}, nil
	}

	return nil, fmt.Errorf("unknown resource: %s", uri)
}

func getGlossaryContent() string {
	glossary := map[string]string{
		"LOD":        "Level of Detail - 3Dモデルの表現の詳細度を示す。LOD0からLOD4まであり、数字が大きいほど詳細。",
		"CityGML":    "3D都市モデルの保存と交換のためのオープンデータモデルおよびXMLベースのフォーマット。",
		"建築物モデル":    "建物や構造物の3D表現。bldgモジュールで定義される。",
		"道路モデル":     "道路などの交通インフラの3D表現。tranモジュールで定義される。",
		"土地利用モデル":   "都市の土地利用の分類と表現。luseモジュールで定義される。",
		"災害リスク":     "自然災害の確率と影響の評価。洪水、土砂災害などのリスク情報。",
		"洪水浸水想定":    "洪水時の予想浸水深と浸水範囲。fldモジュールで定義される。",
		"都市計画決定情報":  "都市計画法に基づく区域区分や用途地域等の情報。urfモジュールで定義される。",
		"メッシュコード":   "日本の標準地域メッシュを識別するコード。CityGMLファイルの分割単位として使用。",
		"空間ID":      "3次元空間を一意に識別するためのID。z/f/x/y形式で表現される。",
		"PLATEAU仕様":  "国土交通省が定める3D都市モデルの標準仕様。年度ごとにバージョンが更新される。",
	}

	var sb strings.Builder
	sb.WriteString("# PLATEAU 用語集\n\n")

	for term, definition := range glossary {
		sb.WriteString(fmt.Sprintf("## %s\n\n%s\n\n", term, definition))
	}

	data, _ := json.MarshalIndent(glossary, "", "  ")
	sb.WriteString("\n## JSON形式\n\n```json\n")
	sb.WriteString(string(data))
	sb.WriteString("\n```\n")

	return sb.String()
}
