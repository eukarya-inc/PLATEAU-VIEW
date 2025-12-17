package main

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/datacatalogv3"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/log"
)

type DatacatalogGeneratorOptions struct {
	OutputToStdout bool
	OutputURL      string // gs://bucket/path for GCS
}

type DatacatalogGenerator struct {
	config         *Config
	pcms           *plateaucms.CMS
	outputToStdout bool
	outputURL      string
}

func NewDatacatalogGenerator(config *Config, opts DatacatalogGeneratorOptions) *DatacatalogGenerator {
	return &DatacatalogGenerator{
		config:         config,
		outputToStdout: opts.OutputToStdout,
		outputURL:      opts.OutputURL,
	}
}

func (g *DatacatalogGenerator) Generate(projectName string) error {
	ctx := context.Background()

	// タイムアウト設定
	ctx, cancel := context.WithTimeout(ctx, 10*time.Minute)
	defer cancel()

	// 標準出力モードでは情報ログを抑制
	if !g.outputToStdout {
		log.Infof("Starting datacatalog generation for: %s", projectName)
		log.Infof("CMS URL: %s", g.config.CMS_BaseURL)
		log.Infof("Cache directory: ./cache")
	}

	// PLATEAU CMSクライアントの初期化
	if err := g.initializePCMS(); err != nil {
		return fmt.Errorf("failed to initialize PLATEAU CMS: %w", err)
	}

	// メタデータの取得とコンテキストへの設定
	if !g.outputToStdout {
		log.Infof("Fetching metadata from CMS...")
	}
	allMetadata, err := g.pcms.AllMetadata(ctx, true)
	if err != nil {
		if !g.outputToStdout {
			log.Warnf("Failed to fetch metadata: %v", err)
		}
		// Continue anyway - some projects may work without metadata
	} else {
		ctx = plateaucms.SetAllCMSMetadataFromContext(ctx, allMetadata)
		if !g.outputToStdout {
			log.Infof("Found %d metadata entries", len(allMetadata))
		}
	}

	// Reposの初期化と設定
	repos := datacatalogv3.NewRepos(g.pcms)
	repos.EnableCache(true) // キャッシュ機能を有効化

	// デバッグ出力を有効化（キャッシュファイル生成を含む）
	repos.EnableDebug(true)

	// Writer設定
	var memWriter *datacatalogv3.MemRepoWriter
	var gcsStorage *datacatalogv3.GCSStorage
	if g.outputToStdout {
		// 標準出力モードではメモリライターを使用
		memWriter = datacatalogv3.NewMemRepoWriter()
		repos.SetWriter(memWriter)
	} else if g.outputURL != "" && strings.HasPrefix(g.outputURL, "gs://") {
		// GCS出力モード
		var err error
		gcsStorage, err = datacatalogv3.NewGCSStorage(ctx, g.outputURL)
		if err != nil {
			return fmt.Errorf("failed to create GCS storage: %w", err)
		}
		defer func() {
			_ = gcsStorage.Close()
		}()
		repos.SetWriter(gcsStorage)
		log.Infof("Output to GCS: %s", g.outputURL)
	}

	// プロジェクト情報の抽出
	year := g.extractYear(projectName)
	isPlateau := g.isPlateau(projectName)

	if !g.outputToStdout {
		log.Infof("Project: %s, Year: %d, IsPlateau: %v", projectName, year, isPlateau)
	}

	// CMSインターフェースの準備（プロジェクト固有のトークンを使用）
	cmsInterface, err := g.getCMSInterface(allMetadata, projectName)
	if err != nil {
		return fmt.Errorf("failed to get CMS interface: %w", err)
	}

	// リポジトリの準備
	if !g.outputToStdout {
		log.Infof("Preparing repository and fetching data from CMS...")
	}
	startTime := time.Now()

	if err := repos.Prepare(ctx, projectName, year, isPlateau, cmsInterface); err != nil {
		return fmt.Errorf("failed to prepare repository: %w", err)
	}

	elapsed := time.Since(startTime)
	if !g.outputToStdout {
		log.Infof("Data fetching and conversion completed in %.2f seconds", elapsed.Seconds())
	}

	// 標準出力モードの場合
	if g.outputToStdout {
		return g.outputToStdoutMode(ctx, repos, projectName, memWriter)
	}

	// 警告出力
	warnings := repos.Warnings(projectName)
	if err := g.outputWarnings(warnings, projectName); err != nil {
		log.Warnf("Failed to output warnings: %v", err)
	}

	// GCS出力モードの場合はローカルファイルの確認をスキップ
	if gcsStorage != nil {
		log.Infof("Cache written to GCS: %s", g.outputURL)
		g.outputStatistics(repos, projectName)
		return nil
	}

	// 通常のファイル出力モード
	// 生成されたファイルの確認
	if err := g.verifyGeneratedFiles(projectName); err != nil {
		return err
	}

	// 統計情報の出力
	g.outputStatistics(repos, projectName)

	return nil
}

func (g *DatacatalogGenerator) outputToStdoutMode(_ context.Context, repos *datacatalogv3.Repos, projectName string, memWriter *datacatalogv3.MemRepoWriter) error {
	// Defensive check for nil writer
	if memWriter == nil {
		return fmt.Errorf("memory writer is nil")
	}

	// Get the JSON data from memory writer
	data, ok := memWriter.GetData(projectName)
	if !ok || len(data) == 0 {
		// If no data, output null
		fmt.Println("null")

		// Still output warnings to stderr
		warnings := repos.Warnings(projectName)
		for _, w := range warnings {
			fmt.Fprintf(os.Stderr, "warning: %s\n", w)
		}
		return nil
	}

	// Output the JSON data to stdout
	fmt.Print(string(data))

	// Output warnings to stderr from memory writer
	if warningData, ok := memWriter.GetWarnings(projectName); ok && len(warningData) > 0 {
		fmt.Fprint(os.Stderr, string(warningData))
	}

	// Also output any additional warnings from repos
	warnings := repos.Warnings(projectName)
	for _, w := range warnings {
		fmt.Fprintf(os.Stderr, "warning: %s\n", w)
	}

	return nil
}

func (g *DatacatalogGenerator) initializePCMS() error {
	// PLATEAU CMSの初期化
	pcms, err := plateaucms.New(plateaucms.Config{
		CMSBaseURL:       g.config.CMS_BaseURL,
		CMSMainToken:     g.config.CMS_Token,
		CMSSystemProject: g.config.CMS_TokenProject,
		// compat
		CMSMainProject: g.config.CMS_SystemProject,
		AdminToken:     g.config.Sidebar_Token,
	})
	if err != nil {
		return fmt.Errorf("failed to create PLATEAU CMS client: %w", err)
	}

	g.pcms = pcms
	return nil
}

func (g *DatacatalogGenerator) getCMSInterface(allMetadata plateaucms.MetadataList, projectName string) (cms.Interface, error) {
	// プロジェクト固有のトークンを取得（URLはグローバル設定を使用）
	token := g.config.CMS_Token // デフォルトはグローバルトークン

	if allMetadata != nil {
		if md, ok := allMetadata.FindDataCatalog(projectName); ok {
			// プロジェクト表示名（空の場合はフォールバック）
			displayName := projectName
			if displayName == "" {
				displayName = fmt.Sprintf("(ProjectID: %s, Alias: %s)", md.ProjectID, md.ProjectAlias)
			}

			if md.CMSAPIKey != "" {
				token = md.CMSAPIKey
				if !g.outputToStdout {
					log.Infof("Using project-specific CMS token for: %s", displayName)
				}
			} else if !g.outputToStdout {
				log.Infof("Using global CMS token for: %s", displayName)
			}
		} else if !g.outputToStdout {
			log.Infof("Using global CMS token for: %s (no metadata found)", projectName)
		}
	}

	cmsClient, err := cms.New(g.config.CMS_BaseURL, token)
	if err != nil {
		return nil, fmt.Errorf("failed to create CMS client: %w", err)
	}

	return cmsClient, nil
}

func (g *DatacatalogGenerator) extractYear(projectName string) int {
	// "plateau-2024" -> 2024
	parts := strings.Split(projectName, "-")
	if len(parts) >= 2 {
		if year, err := strconv.Atoi(parts[len(parts)-1]); err == nil {
			return year
		}
	}
	return time.Now().Year()
}

func (g *DatacatalogGenerator) isPlateau(projectName string) bool {
	return strings.HasPrefix(projectName, "plateau")
}

func (g *DatacatalogGenerator) outputWarnings(warnings []string, projectName string) error {
	if len(warnings) == 0 {
		log.Infof("No warnings generated")
		return nil
	}

	log.Infof("Warnings during generation: %d", len(warnings))

	// 最初の10件の警告を表示
	displayCount := len(warnings)
	if displayCount > 10 {
		displayCount = 10
	}

	for i := 0; i < displayCount; i++ {
		log.Warnf("  - %s", warnings[i])
	}

	if len(warnings) > 10 {
		log.Infof("  ... and %d more warnings (see cache/repo_%s_warnings.txt)",
			len(warnings)-10, projectName)
	}

	return nil
}

func (g *DatacatalogGenerator) verifyGeneratedFiles(projectName string) error {
	// キャッシュディレクトリの作成を確認
	cacheDir := "cache"
	if _, err := os.Stat(cacheDir); os.IsNotExist(err) {
		return fmt.Errorf("cache directory was not created: %s", cacheDir)
	}

	// キャッシュファイルの存在確認
	cacheFile := filepath.Join(cacheDir, fmt.Sprintf("repo_%s.json", projectName))
	warningsFile := filepath.Join(cacheDir, fmt.Sprintf("repo_%s_warnings.txt", projectName))

	// メインキャッシュファイルの確認
	if info, err := os.Stat(cacheFile); err != nil {
		return fmt.Errorf("cache file was not created: %s", cacheFile)
	} else {
		log.Infof("Generated cache file: %s (%.2f MB)",
			cacheFile, float64(info.Size())/1024/1024)
	}

	// 警告ファイルの確認（警告がある場合のみ生成される）
	if info, err := os.Stat(warningsFile); err == nil {
		log.Infof("Generated warnings file: %s (%.2f KB)",
			warningsFile, float64(info.Size())/1024)
	} else {
		log.Infof("No warnings file generated (no warnings)")
	}

	// 個別キャッシュディレクトリの確認
	individualCacheDir := filepath.Join(cacheDir, fmt.Sprintf("cache-datacatalogv3-%s", projectName))
	if info, err := os.Stat(individualCacheDir); err == nil && info.IsDir() {
		// ディレクトリ内のファイル数をカウント
		entries, _ := os.ReadDir(individualCacheDir)
		log.Infof("Generated individual cache files: %d files in %s",
			len(entries), individualCacheDir)
	}

	return nil
}

func (g *DatacatalogGenerator) outputStatistics(repos *datacatalogv3.Repos, projectName string) {
	log.Infof("=====================================")
	log.Infof("Generation statistics for %s:", projectName)
	log.Infof("=====================================")

	repo := repos.Repo(projectName)
	if repo == nil {
		log.Infof("  No repository data available (project may not exist in CMS)")
		log.Infof("  Total warnings: %d", len(repos.Warnings(projectName)))
		log.Infof("=====================================")
		return
	}

	// リポジトリの統計情報を収集
	ctx := context.Background()

	// リポジトリが有効かチェック - use method will check if underlying repo is nil
	areas, err := repo.Areas(ctx, nil)
	if err != nil {
		log.Infof("  Repository data is empty or unavailable")
		log.Infof("  Total warnings: %d", len(repos.Warnings(projectName)))
		log.Infof("=====================================")
		return
	}

	datasets, _ := repo.Datasets(ctx, nil)
	datasetTypes, _ := repo.DatasetTypes(ctx, nil)

	log.Infof("  Total areas: %d", len(areas))
	log.Infof("  Total datasets: %d", len(datasets))
	log.Infof("  Total dataset types: %d", len(datasetTypes))
	log.Infof("  Total warnings: %d", len(repos.Warnings(projectName)))

	// 地域タイプ別の内訳
	prefCount := 0
	cityCount := 0
	wardCount := 0

	for _, area := range areas {
		switch area.GetType() {
		case plateauapi.AreaTypePrefecture:
			prefCount++
		case plateauapi.AreaTypeCity:
			cityCount++
		case plateauapi.AreaTypeWard:
			wardCount++
		}
	}

	log.Infof("  Area breakdown:")
	log.Infof("    - Prefectures: %d", prefCount)
	log.Infof("    - Cities: %d", cityCount)
	log.Infof("    - Wards: %d", wardCount)

	// データセットカテゴリ別の内訳
	plateauCount := 0
	relatedCount := 0
	genericCount := 0

	for _, ds := range datasets {
		switch ds.(type) {
		case *plateauapi.PlateauDataset:
			plateauCount++
		case *plateauapi.RelatedDataset:
			relatedCount++
		case *plateauapi.GenericDataset:
			genericCount++
		}
	}

	log.Infof("  Dataset breakdown:")
	log.Infof("    - PLATEAU datasets: %d", plateauCount)
	log.Infof("    - Related datasets: %d", relatedCount)
	log.Infof("    - Generic datasets: %d", genericCount)
	log.Infof("=====================================")
}
