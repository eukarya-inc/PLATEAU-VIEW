# PLATEAU VIEW データフロー完全ガイド

## 1. システム概要

### 1.1 PLATEAU VIEWとは

PLATEAU VIEWは、国土交通省が推進するPLATEAU（プラトー）プロジェクトの3D都市モデルデータを配信・可視化するためのWebアプリケーションシステムです。本ドキュメントでは、その中核となるデータ配信サーバー（Go言語実装）のデータフローについて詳しく解説します。

主な機能：
- 3D都市モデルデータ（建築物、道路、災害リスク情報など）の配信
- GraphQL APIによる柔軟なデータクエリ
- 地域・年度・データタイプによる高度な検索機能
- リアルタイムデータ更新とキャッシュ管理

### 1.2 主要コンポーネント

#### Re:Earth CMS
- PLATEAU VIEWのコンテンツ管理システム
- 都市データ、メタデータ、プロジェクト設定を管理
- WebhookによるリアルタイムデータSync
- 複数プロジェクト（年度別データなど）の統合管理

#### PLATEAU VIEW Server (Go)
- CMSからデータを取得・変換・キャッシュ
- GraphQL APIエンドポイントの提供
- 高速なインメモリデータストア
- マルチプロジェクト対応のデータ統合

#### GraphQL API
- 柔軟なデータクエリインターフェース
- 階層的な地域データの効率的な検索
- リアルタイムデータ更新対応
- 型安全なスキーマ定義

#### データキャッシュシステム
- インメモリキャッシュ（高速アクセス）
- ディスクキャッシュ（永続化）
- デバッグ用JSONダンプ機能
- 警告・エラーログの記録

### 1.3 データの種類

#### PLATEAU都市モデルデータ
- **建築物（bldg）**: 3D建物モデル、属性情報
- **都市計画（urf）**: 用途地域、都市計画決定情報
- **災害リスク（fld, tnm, lsld）**: 洪水、津波、土砂災害
- **交通（tran, brid）**: 道路、橋梁
- **土地利用（luse）**: 土地利用現況
- **その他**: 植生、都市設備など

#### 関連データセット
- **避難施設（shelter）**: 避難所、避難場所情報
- **ランドマーク（landmark）**: 主要施設、観光地
- **鉄道（railway, station）**: 鉄道路線、駅情報
- **行政界（border）**: 市区町村境界
- **公園（park）**: 都市公園情報

#### 汎用データセット
- **ユースケース（usecase）**: 活用事例データ
- **サンプル（sample）**: デモ・テスト用データ
- カスタムデータセット対応

## 2. アーキテクチャ

### 2.1 全体構成図

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   クライアント    │     │   管理者画面     │     │  外部システム    │
│  (Web/Mobile)   │     │   (Re:Earth)    │     │   (FME等)      │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                        │
         │ GraphQL               │ Admin API             │ Webhook
         ▼                       ▼                        ▼
┌──────────────────────────────────────────────────────────────────┐
│                    PLATEAU VIEW Server (Go)                      │
│  ┌────────────┐  ┌─────────────┐  ┌──────────────────────┐    │
│  │  GraphQL   │  │   Service    │  │    CMS Integration   │    │
│  │  Handler   │  │   Router     │  │      Service         │    │
│  └──────┬─────┘  └──────┬──────┘  └──────────┬───────────┘    │
│         │               │                      │                 │
│         ▼               ▼                      ▼                 │
│  ┌────────────────────────────────────────────────────────┐    │
│  │              DataCatalog Service (v3)                   │    │
│  │  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐  │    │
│  │  │   Fetcher   │  │  Converter   │  │  Repository  │  │    │
│  │  │   (CMS)     │  │  (AllData)   │  │  (InMemory)  │  │    │
│  │  └─────────────┘  └──────────────┘  └──────────────┘  │    │
│  └────────────────────────────────────────────────────────┘    │
│                               │                                  │
│  ┌────────────────────────────┴────────────────────────────┐    │
│  │                    Cache Layer                           │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │    │
│  │  │    Memory    │  │     Disk     │  │    Debug     │  │    │
│  │  │    Cache     │  │    Cache     │  │  JSON/Log    │  │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘  │    │
│  └──────────────────────────────────────────────────────────┘    │
└────────────────────────────────┬─────────────────────────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │    Re:Earth CMS API     │
                    │  ┌─────────┬─────────┐  │
                    │  │ Models  │  Items  │  │
                    │  └─────────┴─────────┘  │
                    └─────────────────────────┘
```

### 2.2 サービス間の関係

#### サービス登録パターン
```go
type Service struct {
    Name           string                    // サービス識別名
    Echo           func(g *echo.Group) error // HTTPルート登録
    Webhook        cmswebhook.Handler        // Webhookハンドラ
    DisableNoCache bool                      // キャッシュ制御
}
```

主要サービス：
- **datacatalog**: データカタログAPI（GraphQL）
- **cmsintegration**: CMS連携・Webhook処理
- **tiles**: タイルデータ配信
- **citygml**: CityGMLファイル処理
- **proxy**: 外部リソースプロキシ

### 2.3 データフローの概要

1. **初期化フェーズ**
   - サーバー起動時にCMSからメタデータ取得
   - 各プロジェクト（年度別データ）の初期化
   - キャッシュデータの読み込み

2. **データ取得フェーズ**
   - CMSからバッチでデータ取得
   - ページネーション処理で大量データ対応
   - エラーリトライとフォールバック

3. **データ変換フェーズ**
   - CMS形式からPlateauAPI形式への変換
   - 地域階層構造の構築
   - データセット関連付け

4. **キャッシュ・配信フェーズ**
   - インメモリストアへの格納
   - GraphQL APIでの配信
   - リアルタイム更新対応

## 3. 初期化プロセス

### 3.1 サーバー起動

#### エントリーポイント（main.go）
```go
func main() {
    // 設定の読み込み
    conf := config.Load()
    
    // Echoサーバーの初期化
    e := echo.New()
    
    // サービスの登録
    services := []service.Service{
        datacatalog.Service(conf),      // データカタログサービス
        cmsintegration.Service(conf),   // CMS連携サービス
        tiles.Service(conf),            // タイル配信サービス
        // ... その他のサービス
    }
    
    // 各サービスの初期化
    for _, s := range services {
        if err := s.Init(e); err != nil {
            log.Fatal(err)
        }
    }
    
    // サーバー起動
    e.Start(":" + conf.Port)
}
```

#### サービス登録パターン（service.go）
```go
type Service struct {
    Name           string                    // サービス識別名
    Echo           func(g *echo.Group) error // HTTPルート登録
    Webhook        cmswebhook.Handler        // Webhookハンドラ
    DisableNoCache bool                      // キャッシュ制御
}

// データカタログサービスの例
func DatacatalogService(conf *Config) Service {
    return Service{
        Name: "datacatalog",
        Echo: func(g *echo.Group) error {
            // GraphQLハンドラの登録
            g.POST("/graphql", graphqlHandler)
            g.GET("/graphql", playgroundHandler)
            return nil
        },
        Webhook: handleDatacatalogWebhook,
    }
}
```

#### 設定の読み込み
```go
type Config struct {
    // サーバー設定
    Port        string `env:"PORT" envDefault:"8080"`
    Environment string `env:"PLATEAU_ENV" envDefault:"development"`
    
    // CMS設定
    CMSBaseURL   string `env:"CMS_BASE_URL"`
    CMSToken     string `env:"CMS_TOKEN"`
    CMSProjects  string `env:"CMS_PROJECTS"` // カンマ区切り
    
    // キャッシュ設定
    CacheDir     string `env:"CACHE_DIR" envDefault:"./cache"`
    CacheEnabled bool   `env:"CACHE_ENABLED" envDefault:"true"`
    
    // デバッグ設定
    Debug        bool   `env:"DEBUG" envDefault:"false"`
    DumpCache    bool   `env:"DUMP_CACHE" envDefault:"false"`
}
```

### 3.2 メタデータの取得

#### plateaucms.AllMetadata()の役割
```go
func (c *CMS) AllMetadata() ([]Metadata, error) {
    // 全プロジェクトのメタデータを取得
    var allMetadata []Metadata
    
    for _, projectID := range c.projectIDs {
        // 各プロジェクトからメタデータ取得
        metadata, err := c.fetchProjectMetadata(projectID)
        if err != nil {
            log.Printf("Failed to fetch metadata for project %s: %v", projectID, err)
            continue
        }
        
        allMetadata = append(allMetadata, metadata)
    }
    
    return allMetadata, nil
}

type Metadata struct {
    ProjectID    string
    ProjectAlias string    // 例: "plateau-2024"
    Year         int       // 例: 2024
    Models       []Model   // City, Dataset, Related, Generic
    UpdatedAt    time.Time
}
```

#### プロジェクト情報の収集
1. **複数年度対応**
   - plateau-2023、plateau-2024など年度別プロジェクト
   - 各プロジェクトは独立したCMSインスタンス
   - 統合的なデータアクセスを提供

2. **モデル定義の取得**
   ```go
   type Model struct {
       ID     string
       Name   string       // "City", "PlateauDataset", etc.
       Fields []Field
       Items  []Item       // 実際のデータアイテム
   }
   ```

3. **認証情報の管理**
   - プロジェクトごとのAPIトークン
   - 環境変数からの読み込み
   - セキュアな保管と利用

### 3.3 リポジトリの準備

#### datacatalogv3.Reposの初期化
```go
type Repos struct {
    repos map[string]*Repo  // projectAlias -> Repo
    mu    sync.RWMutex
}

func NewRepos() *Repos {
    return &Repos{
        repos: make(map[string]*Repo),
    }
}
```

#### 各プロジェクトの準備（Prepare）
```go
func (r *Repos) Prepare(ctx context.Context, cms CMS) error {
    // メタデータの取得
    metadata, err := cms.AllMetadata()
    if err != nil {
        return fmt.Errorf("failed to get metadata: %w", err)
    }
    
    // 各プロジェクトのリポジトリを初期化
    for _, m := range metadata {
        repo := &Repo{
            projectAlias: m.ProjectAlias,
            year:         m.Year,
            cache:        NewCache(m.ProjectAlias),
        }
        
        // 初期データの読み込みまたはフェッチ
        if err := repo.Initialize(ctx, cms); err != nil {
            log.Printf("Failed to initialize repo %s: %v", m.ProjectAlias, err)
            continue
        }
        
        r.repos[m.ProjectAlias] = repo
    }
    
    return nil
}
```

#### キャッシュの有効化
```go
type Cache struct {
    enabled      bool
    dir          string
    memoryCache  *InMemoryCache
    diskCache    *DiskCache
}

func (c *Cache) Initialize() error {
    if !c.enabled {
        return nil
    }
    
    // ディスクキャッシュから既存データを読み込み
    if data, err := c.diskCache.Load(); err == nil {
        // メモリキャッシュに展開
        c.memoryCache.Set(data)
        log.Printf("Loaded cache from disk: %d items", len(data))
    }
    
    // デバッグ用JSONダンプの準備
    if config.DumpCache {
        c.prepareDumpDirectory()
    }
    
    return nil
}
```

#### 初期化フローのまとめ
```
1. main.go起動
   ↓
2. 設定読み込み（環境変数）
   ↓
3. サービス登録
   ↓
4. CMS接続・メタデータ取得
   ↓
5. プロジェクトごとのリポジトリ初期化
   ↓
6. キャッシュシステム起動
   ↓
7. HTTPサーバー開始
```

### 3.4 初期化時のエラーハンドリング

#### リトライ機構
```go
func (r *Repo) InitializeWithRetry(ctx context.Context, cms CMS) error {
    maxRetries := 3
    backoff := time.Second
    
    for i := 0; i < maxRetries; i++ {
        err := r.Initialize(ctx, cms)
        if err == nil {
            return nil
        }
        
        if i < maxRetries-1 {
            log.Printf("Initialization failed (attempt %d/%d): %v", i+1, maxRetries, err)
            time.Sleep(backoff)
            backoff *= 2  // 指数バックオフ
        }
    }
    
    return fmt.Errorf("initialization failed after %d attempts", maxRetries)
}
```

#### 部分的初期化のサポート
- 一部のプロジェクトが失敗しても継続
- 失敗したプロジェクトは後でWebhook経由で更新
- ヘルスチェックで状態を報告

#### ログとモニタリング
```go
type InitializationMetrics struct {
    StartTime       time.Time
    EndTime         time.Time
    ProjectsTotal   int
    ProjectsSuccess int
    ProjectsFailed  int
    Warnings        []string
}
```

## 4. CMSからのデータ取得

### 4.1 CMS接続

#### Re:Earth CMS APIの仕組み
Re:Earth CMSは、PLATEAUデータを管理するヘッドレスCMSシステムです。

```go
// CMS接続の初期化
type CMSClient struct {
    baseURL    string
    token      string
    projectID  string
    httpClient *http.Client
}

func NewCMSClient(baseURL, token, projectID string) *CMSClient {
    return &CMSClient{
        baseURL:    baseURL,
        token:      token,
        projectID:  projectID,
        httpClient: &http.Client{
            Timeout: 30 * time.Second,
        },
    }
}
```

#### 認証とプロジェクト管理
```go
// APIリクエストの認証ヘッダー
func (c *CMSClient) buildRequest(endpoint string) (*http.Request, error) {
    url := fmt.Sprintf("%s/api/projects/%s/%s", c.baseURL, c.projectID, endpoint)
    req, err := http.NewRequest("GET", url, nil)
    if err != nil {
        return nil, err
    }
    
    // 認証トークンの設定
    req.Header.Set("Authorization", "Bearer "+c.token)
    req.Header.Set("Content-Type", "application/json")
    
    return req, nil
}
```

#### モデル構造
CMSには以下の4つの主要モデルが定義されています：

1. **City（都市）モデル**
   ```go
   type CityItem struct {
       ID         string
       CityName   string    // 市区町村名
       CityCode   string    // 市区町村コード
       Prefecture string    // 都道府県名
       PrefCode   string    // 都道府県コード
       CityGML    []Asset   // CityGMLファイル
       MaxLOD     []Asset   // 最大LOD情報
       Metadata   []Asset   // メタデータファイル
   }
   ```

2. **PlateauDataset（PLATEAUデータセット）モデル**
   ```go
   type PlateauFeatureItem struct {
       ID         string
       Name       string
       Type       string    // "bldg", "tran", "urf", etc.
       CityCode   string    // 関連する市区町村コード
       Year       int       // データ年度
       Data       []Asset   // 3Dタイルデータ
       MaxLOD     int       // 最大詳細度
       MinLOD     int       // 最小詳細度
   }
   ```

3. **Related（関連データセット）モデル**
   ```go
   type RelatedItem struct {
       ID          string
       Name        string
       Type        string    // "shelter", "landmark", "railway", etc.
       CityCode    string
       Data        []Asset   // GeoJSON、3Dタイルなど
       Description string    // データの説明
   }
   ```

4. **Generic（汎用データセット）モデル**
   ```go
   type GenericItem struct {
       ID          string
       Name        string
       Type        string    // "usecase", "sample", etc.
       CityCode    string
       Data        []Asset
       Config      JSON      // カスタム設定
   }
   ```

### 4.2 データ取得処理

#### GetAll()によるバッチ取得
```go
func (c *CMSClient) GetAll(ctx context.Context) (*AllData, error) {
    allData := &AllData{
        Cities:   []CityItem{},
        Plateau:  []PlateauFeatureItem{},
        Related:  []RelatedItem{},
        Generic:  []GenericItem{},
    }
    
    // 並行取得で高速化
    var wg sync.WaitGroup
    var mu sync.Mutex
    errs := make(chan error, 4)
    
    // City データの取得
    wg.Add(1)
    go func() {
        defer wg.Done()
        cities, err := c.fetchAllItems("cities", &CityItem{})
        if err != nil {
            errs <- fmt.Errorf("failed to fetch cities: %w", err)
            return
        }
        mu.Lock()
        allData.Cities = cities
        mu.Unlock()
    }()
    
    // 同様にPlateau、Related、Genericも並行取得
    // ...
    
    wg.Wait()
    close(errs)
    
    // エラーチェック
    for err := range errs {
        if err != nil {
            return nil, err
        }
    }
    
    return allData, nil
}
```

#### ページネーション処理
CMSは大量データに対してページネーション機能を提供します：

```go
func (c *CMSClient) fetchAllItems(modelID string, itemType interface{}) ([]interface{}, error) {
    var allItems []interface{}
    page := 1
    perPage := 100
    
    for {
        // ページごとにデータ取得
        items, totalCount, err := c.fetchPage(modelID, page, perPage)
        if err != nil {
            return nil, fmt.Errorf("failed to fetch page %d: %w", page, err)
        }
        
        allItems = append(allItems, items...)
        
        // 次のページがあるかチェック
        if len(allItems) >= totalCount {
            break
        }
        
        page++
        
        // レートリミット対策
        time.Sleep(100 * time.Millisecond)
    }
    
    return allItems, nil
}

func (c *CMSClient) fetchPage(modelID string, page, perPage int) ([]interface{}, int, error) {
    endpoint := fmt.Sprintf("models/%s/items?page=%d&perPage=%d", modelID, page, perPage)
    
    req, err := c.buildRequest(endpoint)
    if err != nil {
        return nil, 0, err
    }
    
    resp, err := c.httpClient.Do(req)
    if err != nil {
        return nil, 0, err
    }
    defer resp.Body.Close()
    
    if resp.StatusCode != http.StatusOK {
        return nil, 0, fmt.Errorf("unexpected status: %d", resp.StatusCode)
    }
    
    // レスポンスのパース
    var result struct {
        Items      []json.RawMessage `json:"items"`
        TotalCount int               `json:"totalCount"`
    }
    
    if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
        return nil, 0, err
    }
    
    // 型変換処理
    // ...
    
    return items, result.TotalCount, nil
}
```

#### エラーハンドリング
```go
type CMSError struct {
    Operation string
    ModelID   string
    Err       error
    Retry     bool
}

func (e *CMSError) Error() string {
    return fmt.Sprintf("CMS %s error for %s: %v", e.Operation, e.ModelID, e.Err)
}

// リトライ可能なエラーの判定
func isRetryableError(err error) bool {
    if err == nil {
        return false
    }
    
    // ネットワークエラー
    if netErr, ok := err.(net.Error); ok && netErr.Temporary() {
        return true
    }
    
    // HTTP 429 (Too Many Requests) や 503 (Service Unavailable)
    if httpErr, ok := err.(*HTTPError); ok {
        return httpErr.StatusCode == 429 || httpErr.StatusCode == 503
    }
    
    return false
}
```

### 4.3 CMSデータ構造

#### データアイテムの共通フィールド
```go
type BaseItem struct {
    ID          string          `json:"id"`
    CreatedAt   time.Time       `json:"createdAt"`
    UpdatedAt   time.Time       `json:"updatedAt"`
    PublishedAt *time.Time      `json:"publishedAt"`
    Version     int             `json:"version"`
    Status      string          `json:"status"` // "draft", "published"
}
```

#### アセット管理
```go
type Asset struct {
    ID          string `json:"id"`
    URL         string `json:"url"`
    Name        string `json:"name"`
    Size        int64  `json:"size"`
    ContentType string `json:"contentType"`
}

// アセットURLの解決
func (a *Asset) GetFullURL(baseURL string) string {
    if strings.HasPrefix(a.URL, "http") {
        return a.URL
    }
    return baseURL + "/assets/" + a.ID
}
```

#### データ取得の並列処理
```go
// GetAll()内での並列取得の実装
cityItemsChan := lo.Async2(func() ([]*CityItem, error) {
    return c.GetCityItems(ctx, c.project, featureTypes.Plateau)
})

relatedItemsChan := lo.Async2(func() ([]*RelatedItem, error) {
    return c.GetRelatedItems(ctx, c.project, featureTypes.Related)
})

genericItemsChan := lo.Async2(func() ([]*GenericItem, error) {
    return c.GetGenericItems(ctx, c.project)
})

// 各featureTypeごとの並列取得
featureItemsChans := make([]<-chan lo.Tuple3[string, []*PlateauFeatureItem, error], 0, len(all.FeatureTypes.Plateau))
for _, featureType := range all.FeatureTypes.Plateau {
    featureType := featureType
    if featureType.MinYear > 0 && c.year < featureType.MinYear {
        continue
    }
    
    featureItemsChan := lo.Async3(func() (string, []*PlateauFeatureItem, error) {
        res, err := c.GetPlateauItems(ctx, c.project, featureType.Code)
        return featureType.Code, res, err
    })
    featureItemsChans = append(featureItemsChans, featureItemsChan)
}
```

#### キャッシュ管理
```go
// キャッシュの読み込みと保存
func loadCache[T any](cachePath, key string) (t T, _ error) {
    _ = os.MkdirAll(cachePath, 0755)
    
    f, err := os.Open(filepath.Join(cachePath, key+".json"))
    if err != nil {
        if os.IsNotExist(err) {
            return
        }
        return t, fmt.Errorf("failed to open cache file: %w", err)
    }
    defer f.Close()
    
    var v T
    if err = json.NewDecoder(f).Decode(&v); err != nil {
        return
    }
    
    return v, nil
}

func saveCache(cachePath, key string, content any) error {
    _ = os.MkdirAll(cachePath, 0755)
    
    f, err := os.Create(filepath.Join(cachePath, key+".json"))
    if err != nil {
        return fmt.Errorf("failed to create cache file: %w", err)
    }
    defer f.Close()
    
    if err = json.NewEncoder(f).Encode(content); err != nil {
        return fmt.Errorf("failed to encode cache content: %w", err)
    }
    
    return nil
}
```

#### AllData構造
```go
type AllData struct {
    Name                  string
    Year                  int
    CMSInfo               CMSInfo
    PlateauSpecs          []plateauapi.PlateauSpecSimple
    FeatureTypes          FeatureTypes
    City                  []*CityItem
    Plateau               map[string][]*PlateauFeatureItem // featureType -> items
    Related               []*RelatedItem
    Generic               []*GenericItem
    Sample                []*PlateauFeatureItem
    GeospatialjpDataItems []*GeospatialjpDataItem
}
```

## 5. データ変換処理

### 5.1 変換の概要（conv.go）

#### AllData.Into()メソッド
CMSから取得したデータをGraphQL API用の形式に変換する中核処理です。

```go
func (all *AllData) Into() (res *plateauapi.InMemoryRepoContext, warning []string) {
    if all == nil {
        warning = append(warning, "data is nil")
        return
    }
    
    res = &plateauapi.InMemoryRepoContext{
        Name:     all.Name,
        Areas:    plateauapi.Areas{},
        Datasets: plateauapi.Datasets{},
    }
    res.PlateauSpecs = plateauapi.PlateauSpecsFrom(all.PlateauSpecs)
    res.DatasetTypes = all.FeatureTypes.ToDatasetTypes(res.PlateauSpecs)
    
    ic := newInternalContext()
    ic.cmsinfo = all.CMSInfo
    ic.regYear = all.Year
    
    // 変換処理の流れ
    // 1. 都道府県・市区町村の変換
    // 2. 区（Ward）の抽出
    // 3. PLATEAUデータセットの変換
    // 4. 関連データセットの変換
    // 5. 汎用データセットの変換
    // 6. CityGMLファイルの処理
    
    return
}
```

#### 内部コンテキスト（internalContext）
変換処理全体で共有される情報を管理：

```go
type internalContext struct {
    cmsinfo           CMSInfo
    regYear           int
    prefByCode        map[plateauapi.AreaCode]*Prefecture
    cityByCode        map[plateauapi.AreaCode]*City
    areaContextByCode map[plateauapi.AreaCode]*areaContext
    layerNamesForType map[string]LayerNames
    years             []int
}

type areaContext struct {
    CityItem *CityItem
    Pref     *plateauapi.Prefecture
    City     *plateauapi.City
}
```

#### 警告の収集
変換処理中に発生した問題を警告として収集し、後でログ出力やデバッグに使用：

```go
var warning []string

// 警告の追加例
if area == nil {
    warning = append(warning, fmt.Sprintf("plateau %s: city not found: %s", ds.ID, ds.City))
    continue
}
```

### 5.2 エリアデータの変換

#### 都道府県（Prefecture）の生成
```go
func (c *CityItem) ToPrefecture() *plateauapi.Prefecture {
    if c == nil || c.Prefecture == "" || c.CityCode == "" {
        return nil
    }
    
    code := plateauapi.AreaCode(c.CityCode[:2])
    pref := plateauapi.NewPrefecture(code, c.Prefecture, nil)
    return &pref
}
```

#### 市区町村（City）の生成
```go
func (c *CityItem) ToCity() *plateauapi.City {
    if c == nil || c.CityName == "" || c.CityCode == "" {
        return nil
    }
    
    code := plateauapi.AreaCode(c.CityCode)
    if c.IsMerged() {
        code = plateauapi.AreaCode(c.SubCityCode)
    }
    
    city := plateauapi.NewCity(
        code,
        c.CityName,
        c.CityNameEn,
        plateauapi.AreaCode(c.CityCode[:2]), // parent prefecture
        c.Description(),
    )
    
    return &city
}
```

#### 区（Ward）の抽出
PLATEAUデータから区情報を抽出：

```go
func getWards(items []*PlateauFeatureItem, ic *internalContext) (res []*plateauapi.Ward, warning []string) {
    for _, ds := range items {
        area := ic.AreaContext(ds.City)
        if area == nil {
            warning = append(warning, fmt.Sprintf("plateau %s: city not found: %s", ds.ID, ds.City))
            continue
        }
        
        wards := ds.toWards(area.Pref, area.City)
        res = append(res, wards...)
    }
    
    return
}

// PlateauFeatureItemから区を抽出
func (c *PlateauFeatureItem) toWards(pref *plateauapi.Prefecture, city *plateauapi.City) []*plateauapi.Ward {
    var wards []*plateauapi.Ward
    
    for _, item := range c.Items {
        if item.WardCode != "" && item.WardName != "" {
            ward := plateauapi.NewWard(
                plateauapi.AreaCode(item.WardCode),
                item.WardName,
                item.WardNameEn,
                city.Code,
                pref.Code,
            )
            wards = append(wards, &ward)
        }
    }
    
    return wards
}
```

### 5.3 データセットの変換

#### PLATEAUデータセットの変換
```go
func convertPlateau(
    items []*PlateauFeatureItem,
    featureType string,
    specs []plateauapi.PlateauSpec,
    dts map[string]plateauapi.DatasetType,
    fts map[string]*FeatureType,
    ic *internalContext,
) (res []plateauapi.Dataset, warning []string) {
    dt := dts[featureType]
    if dt == nil {
        warning = append(warning, fmt.Sprintf("unknown feature type: %s", featureType))
        return
    }
    
    for _, item := range items {
        datasets, w := item.ToDatasets(dt, specs, fts[featureType], ic)
        warning = append(warning, w...)
        res = append(res, datasets...)
    }
    
    return
}
```

#### 関連データセットの変換
```go
func convertRelated(
    items []*RelatedItem,
    dts []plateauapi.DatasetType,
    ic *internalContext,
) (res []plateauapi.Dataset, warning []string) {
    dtmap := lo.SliceToMap(dts, func(dt plateauapi.DatasetType) (string, plateauapi.DatasetType) {
        return dt.GetCode(), dt
    })
    
    for _, item := range items {
        for code, data := range item.Items {
            dt := dtmap[code]
            if dt == nil {
                warning = append(warning, fmt.Sprintf("related %s: unknown type: %s", item.ID, code))
                continue
            }
            
            dataset := data.ToDataset(dt, item, ic)
            if dataset != nil {
                res = append(res, dataset)
            }
        }
    }
    
    return
}
```

#### 汎用データセットの変換
```go
func convertGeneric(
    items []*GenericItem,
    dts []plateauapi.DatasetType,
    ic *internalContext,
) (res []plateauapi.Dataset, warning []string) {
    for _, item := range items {
        // TypeまたはTypeEnでデータセットタイプを特定
        var dt plateauapi.DatasetType
        for _, d := range dts {
            if d.GetCode() == item.Type || d.GetCode() == item.TypeEn {
                dt = d
                break
            }
        }
        
        if dt == nil {
            warning = append(warning, fmt.Sprintf("generic %s: unknown type: %s", item.ID, item.Type))
            continue
        }
        
        dataset := item.ToDataset(dt, ic)
        if dataset != nil {
            res = append(res, dataset)
        }
    }
    
    return
}
```

### 5.4 データ間の関連付け

#### エリアとデータセットの関係
```go
// internalContextでの管理
func (ic *internalContext) Add(cityItem *CityItem, pref *plateauapi.Prefecture, city *plateauapi.City) {
    ic.prefByCode[pref.Code] = pref
    ic.cityByCode[city.Code] = city
    ic.areaContextByCode[city.Code] = &areaContext{
        CityItem: cityItem,
        Pref:     pref,
        City:     city,
    }
}

// エリアコンテキストの取得
func (ic *internalContext) AreaContext(cityID string) *areaContext {
    // CMS IDから市区町村コードを解決
    for _, ac := range ic.areaContextByCode {
        if ac.CityItem.ID == cityID {
            return ac
        }
    }
    return nil
}
```

#### 階層構造の構築
```go
// 都道府県 → 市区町村 → 区の階層構造を構築
// 1. CityItemから都道府県・市区町村を生成
for _, cityItem := range all.City {
    pref, city := cityItem.ToPrefecture(), cityItem.ToCity()
    if pref == nil || city == nil {
        continue
    }
    
    ic.Add(cityItem, pref, city)
    
    // 重複チェックして追加
    if res.Areas.FindByCodeAndType(pref.Code, plateauapi.AreaTypePrefecture) == nil {
        res.Areas.Append(plateauapi.AreaTypePrefecture, []plateauapi.Area{pref})
    }
    
    if res.Areas.FindByCodeAndType(city.Code, plateauapi.AreaTypeCity) == nil {
        res.Areas.Append(plateauapi.AreaTypeCity, []plateauapi.Area{city})
    }
}

// 2. PLATEAUデータから区を抽出
for _, ft := range res.DatasetTypes[plateauapi.DatasetTypeCategoryPlateau] {
    wards, w := getWards(all.Plateau[ft.GetCode()], ic)
    warning = append(warning, w...)
    ic.AddWards(wards)
    res.Areas.Append(plateauapi.AreaTypeWard, 
        lo.Map(wards, func(w *plateauapi.Ward, _ int) plateauapi.Area { return w }))
}
```

#### インデックスの作成
変換処理の最後に、高速検索のためのインデックスが自動的に作成されます（InMemoryリポジトリで詳述）。

## 6. InMemoryリポジトリ

### 6.1 データ構造

#### InMemoryRepoContext
すべてのデータをメモリ上に保持する中核構造：

```go
type InMemoryRepoContext struct {
    Name         string                 `json:"name"`
    Areas        Areas                  `json:"areas"`          // 地域データ
    DatasetTypes DatasetTypes           `json:"datasetTypes"`   // データセットタイプ
    Datasets     Datasets               `json:"datasets"`       // データセット
    PlateauSpecs []PlateauSpec          `json:"plateauSpecs"`   // PLATEAU仕様
    Years        []int                  `json:"years"`          // 対応年度
    CityGML      map[ID]*CityGMLDataset `json:"cityGML"`        // CityGMLデータ
}
```

#### Areas（地域データ）
階層構造を持つ地域データの管理：

```go
type Areas map[AreaType][]Area

// AreaTypeごとに地域を管理
const (
    AreaTypePrefecture AreaType = "prefecture" // 都道府県
    AreaTypeCity       AreaType = "city"       // 市区町村
    AreaTypeWard       AreaType = "ward"       // 区
)

// 地域の検索
func (a Areas) FindByCodeAndType(code AreaCode, areaType AreaType) Area {
    areas := a[areaType]
    for _, area := range areas {
        if area.GetCode() == code {
            return area
        }
    }
    return nil
}
```

#### Datasets（データセット）
カテゴリ別にデータセットを管理：

```go
type Datasets map[DatasetTypeCategory][]Dataset

// データセットカテゴリ
const (
    DatasetTypeCategoryPlateau DatasetTypeCategory = "plateau"  // PLATEAUデータ
    DatasetTypeCategoryRelated DatasetTypeCategory = "related"  // 関連データ
    DatasetTypeCategoryGeneric DatasetTypeCategory = "generic"  // 汎用データ
)
```

#### DatasetTypes（データセットタイプ）
データセットの種類を定義：

```go
type DatasetTypes map[DatasetTypeCategory][]DatasetType

// 例：建築物、都市計画、災害リスクなど
```

### 6.2 インデックス構築

#### areasForDataTypesマップ
データセットタイプごとに、どの地域にデータがあるかを高速検索するためのインデックス：

```go
func areasForDatasetTypes(ds []Dataset) map[string]map[AreaCode]bool {
    // true -> 最詳細レベル, false -> 最詳細ではない
    res := make(map[string]map[AreaCode]bool)
    
    for _, d := range ds {
        datasetTypeCode := d.GetTypeCode()
        
        codes := areaCodesFrom(d)           // 関連するすべての地域コード
        code := mostDetailedAreaCodeFrom(d) // 最詳細レベルの地域コード
        
        for _, c := range codes {
            mostDetailed := code != nil && c == *code
            if _, ok := res[datasetTypeCode]; !ok {
                res[datasetTypeCode] = make(map[AreaCode]bool)
            }
            if _, ok := res[datasetTypeCode][c]; !ok || mostDetailed {
                res[datasetTypeCode][c] = mostDetailed
            }
        }
    }
    
    return res
}
```

実際のデータ例：
```go
// 建築物データの場合
"bldg": {
    "04": false,     // 宮城県（最詳細ではない）
    "04100": false,  // 仙台市（最詳細ではない）
    "04101": true,   // 青葉区（最詳細）
    "04102": true,   // 宮城野区（最詳細）
    // ...
}

// 都市計画データの場合
"urf": {
    "04": false,     // 宮城県（最詳細ではない）
    "04100": true,   // 仙台市（最詳細）
}
```

#### areasWithoutDatasetセット
データセットを持たない地域のIDを記録：

```go
func areasWithoutDataset(ds Datasets, areas Areas) map[ID]struct{} {
    res := make(map[ID]struct{})
    
    for _, a := range areas.All() {
        if a == nil {
            continue
        }
        
        found := false
        for _, d := range ds.All() {
            codes := areaCodesFrom(d)
            if lo.Contains(codes, a.GetCode()) {
                found = true
                continue
            }
        }
        
        if !found {
            res[a.GetID()] = struct{}{}
        }
    }
    
    return res
}
```

#### 最詳細レベルの判定
データセットがどの地域レベルで管理されているかを判定：

```go
func mostDetailedAreaCodeFrom(d Dataset) *AreaCode {
    switch d := d.(type) {
    case *PlateauDataset:
        // PLATEAUデータは区レベルが最詳細の場合が多い
        if d.WardCode != nil {
            code := AreaCode(*d.WardCode)
            return &code
        }
        if d.CityCode != nil {
            code := AreaCode(*d.CityCode)
            return &code
        }
    case *RelatedDataset:
        // 関連データは通常市レベル
        if d.CityCode != nil {
            code := AreaCode(*d.CityCode)
            return &code
        }
    // ...
    }
    return nil
}
```

### 6.3 キャッシュシステム

#### メモリキャッシュ
InMemoryRepoは全データをメモリに保持し、高速アクセスを実現：

```go
type InMemoryRepo struct {
    ctx                 *InMemoryRepoContext
    areasForDataTypes   map[string]map[AreaCode]bool
    areasWithoutDataset map[ID]struct{}
}

func NewInMemoryRepo(ctx *InMemoryRepoContext) *InMemoryRepo {
    r := &InMemoryRepo{}
    r.SetContext(ctx)
    return r
}

func (c *InMemoryRepo) SetContext(ctx *InMemoryRepoContext) {
    c.ctx = ctx
    // インデックスの構築
    c.areasForDataTypes = areasForDatasetTypes(ctx.Datasets.All())
    c.areasWithoutDataset = areasWithoutDataset(ctx.Datasets, ctx.Areas)
}
```

#### ディスクキャッシュ
datacatalogv3のキャッシュシステムと連携：

```go
// repo.goでの実装
type repoCacheValue struct {
    InMemoryRepoContext *plateauapi.InMemoryRepoContext
    Warnings            []string
}

func (r *Repo) saveCache() error {
    if !r.cache || r.inmemoryContext == nil {
        return nil
    }
    
    cv := repoCacheValue{
        InMemoryRepoContext: r.inmemoryContext,
        Warnings:            r.warnings,
    }
    
    return r.saver.Save(cv)
}
```

#### デバッグ出力（JSON、warnings.txt）
開発時のデバッグ用にキャッシュデータをJSON形式で出力：

```go
// デバッグ用のJSON出力（cache/repo_plateau-2024.json）
{
    "name": "plateau-2024",
    "areas": {
        "prefecture": [...],
        "city": [...],
        "ward": [...]
    },
    "datasets": {
        "plateau": [...],
        "related": [...],
        "generic": [...]
    },
    "datasetTypes": {...},
    "years": [2023, 2024]
}

// 警告ログ（cache/warnings.txt）
plateau xxx: city not found: yyy
related zzz: unknown type: aaa
```

キャッシュファイルの場所：
- メインキャッシュ: `cache/repo_plateau-{year}.json`
- 警告ログ: `cache/warnings.txt`
- 個別データキャッシュ: `cache/cache-datacatalogv3-plateau-{year}/`

## 7. GraphQL API

### 7.1 スキーマ定義

#### schema.graphqlの構造
PLATEAU GraphQL APIのスキーマは、地域データとデータセットを中心に設計されています：

```graphql
# 基本インターフェース
interface Node {
  id: ID!
}

# 地域インターフェース
interface Area implements Node {
  id: ID!
  type: AreaType!
  code: AreaCode!
  name: String!
  datasets(input: DatasetsInput): [Dataset!]!
  parentId: ID
  parent: Area
  children: [Area!]!
}

# 地域の種類
enum AreaType {
  PREFECTURE  # 都道府県
  CITY        # 市町村
  WARD        # 区（政令指定都市のみ）
}
```

#### 主要な型（Area、Dataset、DatasetType）

**Area型の階層構造**：
```graphql
type Prefecture implements Area & Node {
  id: ID!
  type: AreaType!
  code: AreaCode!
  name: String!
  cities: [City!]!
  datasets(input: DatasetsInput): [Dataset!]!
  # ...
}

type City implements Area & Node {
  id: ID!
  type: AreaType!
  code: AreaCode!
  name: String!
  prefecture: Prefecture
  wards: [Ward!]!
  datasets(input: DatasetsInput): [Dataset!]!
  # ...
}

type Ward implements Area & Node {
  id: ID!
  type: AreaType!
  code: AreaCode!
  name: String!
  city: City!
  prefecture: Prefecture!
  datasets(input: DatasetsInput): [Dataset!]!
  # ...
}
```

**Dataset型の種類**：
```graphql
interface Dataset implements Node {
  id: ID!
  name: String!
  description: String
  year: Int!
  groups: [String!]
  prefectureId: ID
  prefectureCode: AreaCode
  cityId: ID
  cityCode: AreaCode
  wardId: ID
  wardCode: AreaCode
  prefecture: Prefecture
  city: City
  ward: Ward
  type: DatasetType!
  items: [DatasetItem!]!
}

# PLATEAU都市モデルデータセット
type PlateauDataset implements Dataset & Node {
  # ... Dataset fields
  subname: String
  suborder: Int
  plateauSpecMinorId: ID!
  # ...
}

# 関連データセット
type RelatedDataset implements Dataset & Node {
  # ... Dataset fields
  # ...
}

# 汎用データセット  
type GenericDataset implements Dataset & Node {
  # ... Dataset fields
  # ...
}
```

#### クエリとリゾルバー

**主要なクエリ**：
```graphql
type Query {
  # IDによる取得
  node(id: ID!): Node
  nodes(ids: [ID!]!): [Node]!
  
  # 地域の検索
  area(code: AreaCode!): Area
  areas(input: AreasInput): [Area!]!
  
  # データセットタイプの検索
  datasetTypes(input: DatasetTypesInput): [DatasetType!]!
  
  # データセットの検索
  datasets(input: DatasetsInput): [Dataset!]!
  
  # PLATEAU仕様の取得
  plateauSpecs: [PlateauSpec!]!
  
  # 対応年度の取得
  years: [Int!]!
}
```

### 7.2 リゾルバーの実装

#### Areasクエリの処理フロー
```go
// schema.resolvers.go
func (r *queryResolver) Areas(ctx context.Context, input *AreasInput) ([]Area, error) {
    return r.Repo.Areas(ctx, input)
}

// inmemory.go
func (c *InMemoryRepo) Areas(ctx context.Context, input *AreasInput) (res []Area, _ error) {
    inp := lo.FromPtr(input)
    types := c.getDatasetTypeCodes(inp.DatasetTypes, inp.Categories)
    
    var codes []AreaCode
    if inp.DatasetTypes != nil {
        // areasForDataTypesインデックスを使用
        for _, t := range types {
            for k, v := range c.areasForDataTypes[t] {
                if input.IncludeParents != nil && *input.IncludeParents || v {
                    codes = append(codes, k)
                }
            }
        }
    }
    
    res = c.ctx.Areas.Filter(func(a Area) bool {
        if !filterArea(a, inp, c.areasWithoutDataset) {
            return false
        }
        
        if inp.DatasetTypes != nil && !lo.Contains(codes, a.GetCode()) {
            return false
        }
        
        return true
    })
    return
}
```

#### フィルタリングロジック
```go
func filterArea(a Area, input AreasInput, areasWithoutDataset map[ID]struct{}) bool {
    // 地域タイプのフィルタリング
    if len(input.AreaTypes) > 0 {
        if !lo.Contains(input.AreaTypes, a.GetType()) {
            return false
        }
    }
    
    // 親地域コードのフィルタリング
    if input.ParentCode != nil {
        parent := a.GetParent()
        if !input.Deep {
            // 直接の子のみ
            if parent == nil || parent.GetCode() != *input.ParentCode {
                return false
            }
        } else {
            // 間接的な子も含む
            found := false
            for p := parent; p != nil; p = p.GetParent() {
                if p.GetCode() == *input.ParentCode {
                    found = true
                    break
                }
            }
            if !found {
                return false
            }
        }
    }
    
    // 空の地域の除外
    if !input.IncludeEmpty {
        if _, empty := areasWithoutDataset[a.GetID()]; empty {
            return false
        }
    }
    
    // 検索文字列
    if len(input.SearchTokens) > 0 {
        name := a.GetName()
        for _, token := range input.SearchTokens {
            if !strings.Contains(name, token) {
                return false
            }
        }
    }
    
    return true
}
```

#### 動的フィールドの解決
GraphQLの特徴として、クライアントが必要なフィールドのみを要求できます：

```go
// 例：市のリゾルバー
type cityResolver struct{ *Resolver }

// 都道府県フィールドが要求された場合のみ実行
func (r *cityResolver) Prefecture(ctx context.Context, obj *City) (*Prefecture, error) {
    return to[*Prefecture](r.Repo.Node(ctx, obj.PrefectureID))
}

// 区フィールドが要求された場合のみ実行
func (r *cityResolver) Wards(ctx context.Context, obj *City) ([]*Ward, error) {
    areas, err := r.Repo.Areas(ctx, &AreasInput{
        ParentCode: lo.ToPtr(obj.Code),
    })
    if err != nil {
        return nil, err
    }
    
    return lo.FilterMap(areas, func(a Area, _ int) (*Ward, bool) {
        if m, ok := a.(*Ward); ok {
            return m, ok
        }
        return nil, false
    }), nil
}

// データセットフィールドが要求された場合のみ実行
func (r *cityResolver) Datasets(ctx context.Context, obj *City, input *DatasetsInput) ([]Dataset, error) {
    if input == nil {
        input = &DatasetsInput{}
    }
    input.AreaCodes = []AreaCode{obj.Code}
    return r.Repo.Datasets(ctx, input)
}
```

### 7.3 フィルタリングパラメータ

#### parentCodeの動作
親地域を指定して子地域を検索：

```go
// 例：宮城県（04）の直接の子を取得
{
  areas(input: { parentCode: "04" }) {
    code
    name
  }
}
// 結果：仙台市（04100）など

// deepオプションで間接的な子も取得
{
  areas(input: { parentCode: "04", deep: true }) {
    code
    name
  }
}
// 結果：仙台市（04100）＋各区（04101-04105）
```

#### datasetTypesの動作
特定のデータセットタイプを持つ地域を検索：

```go
// 建築物データを持つ地域を検索
{
  areas(input: { datasetTypes: ["bldg"] }) {
    code
    name
  }
}
// 結果：建築物データを持つ区のみ（最詳細レベル）

// includeParentsで親地域も含める
{
  areas(input: { 
    datasetTypes: ["bldg"],
    includeParents: true 
  }) {
    code
    name
  }
}
// 結果：区＋その親の市・県
```

#### includeParentsとdeepの違い
- **includeParents**: データセットを持つ地域の親を結果に含める（上方向の拡張）
- **deep**: 検索範囲を間接的な子まで広げる（下方向の拡張）

#### その他のオプション
- **areaTypes**: 地域タイプ（PREFECTURE/CITY/WARD）でフィルタリング
- **searchTokens**: 地域名での文字列検索（AND条件）
- **includeEmpty**: データセットを持たない地域も含める
- **categories**: データセットカテゴリ（PLATEAU/RELATED/GENERIC）でフィルタリング

## 8. データの階層構造

### 8.1 地域の階層

#### 都道府県 → 市区町村 → 区
日本の行政区分に基づく3層構造：

```
都道府県（Prefecture）
├── 市区町村（City）
│   ├── 一般市町村（区なし）
│   └── 政令指定都市
│       ├── 区1（Ward）
│       ├── 区2（Ward）
│       └── ...
```

実例：
```
宮城県（04）
├── 仙台市（04100）- 政令指定都市
│   ├── 青葉区（04101）
│   ├── 宮城野区（04102）
│   ├── 若林区（04103）
│   ├── 太白区（04104）
│   └── 泉区（04105）
├── 石巻市（04202）- 一般市
├── 塩竈市（04203）- 一般市
└── ...
```

#### 政令指定都市の特殊性
政令指定都市のみ区を持ち、データ管理が特殊：

```go
// 政令指定都市の判定
func (c *City) IsDesignatedCity() bool {
    // 区を持つ = 政令指定都市
    return len(c.Wards) > 0
}

// 政令指定都市の場合、建築物データは区レベルで管理
// 一般市の場合、建築物データは市レベルで管理
```

日本の政令指定都市（20市）：
- 札幌市、仙台市、さいたま市、千葉市、横浜市、川崎市、相模原市
- 新潟市、静岡市、浜松市、名古屋市、京都市、大阪市、堺市、神戸市
- 岡山市、広島市、北九州市、福岡市、熊本市

### 8.2 データセットの配置

#### 建築物（bldg）：区レベル
政令指定都市では建築物データは区単位で管理：

```json
// 仙台市の例
{
  "青葉区": {
    "datasets": ["bldg_04101_sendai-shi_2023_3dtiles_1_op"]
  },
  "宮城野区": {
    "datasets": ["bldg_04102_sendai-shi_2023_3dtiles_1_op"]
  }
  // 仙台市レベルには建築物データなし
}
```

理由：
- データ量が膨大（数万〜数十万の建物）
- 区ごとに分割することで管理・配信を効率化
- 3Dタイルの生成・更新が区単位で可能

#### その他：主に市レベル
他のデータセットは主に市レベルで管理：

```json
// 仙台市の例
{
  "仙台市": {
    "datasets": [
      "urf_04100_sendai-shi_2023",     // 都市計画
      "fld_04100_sendai-shi_2023",     // 洪水浸水想定
      "tnm_04100_sendai-shi_2023",     // 津波浸水想定
      "lsld_04100_sendai-shi_2023",    // 土砂災害警戒区域
      "brid_04100_sendai-shi_2023"     // 橋梁
    ]
  }
}
```

理由：
- データ量が建築物より少ない
- 行政計画・災害想定は市単位で策定
- 市全体での統一的な管理が必要

#### 階層による影響
データの配置階層により、検索・フィルタリングの挙動が変わる：

1. **最詳細レベルの概念**
   ```go
   // 建築物の場合：区が最詳細
   "bldg" -> ["04101", "04102", ...] // 区コードのみ
   
   // 都市計画の場合：市が最詳細
   "urf" -> ["04100", "04201", ...] // 市コードのみ
   ```

2. **includeParentsの必要性**
   ```graphql
   # 建築物データがある「市」を検索したい場合
   {
     areas(input: {
       parentCode: "04",
       datasetTypes: ["bldg"],
       includeParents: true  # これがないと仙台市が含まれない
     }) {
       code
       name
     }
   }
   ```

### 8.3 フィルタリングへの影響

#### 最詳細レベルの概念
areasForDataTypesマップでの管理：

```go
// areasForDatasetTypes関数の動作
func areasForDatasetTypes(ds []Dataset) map[string]map[AreaCode]bool {
    res := make(map[string]map[AreaCode]bool)
    
    for _, d := range ds {
        datasetTypeCode := d.GetTypeCode() // "bldg", "urf", etc.
        
        // データセットに関連する全地域コード
        codes := areaCodesFrom(d) // [県, 市, 区（あれば）]
        
        // 最詳細レベルの地域コード
        code := mostDetailedAreaCodeFrom(d) // 区 or 市
        
        for _, c := range codes {
            mostDetailed := code != nil && c == *code
            res[datasetTypeCode][c] = mostDetailed
        }
    }
    
    return res
}
```

#### 親子関係の考慮
データセットの検索時の親子関係：

```go
// 親から子を検索
func searchFromParent(parentCode string, datasetType string) {
    // 1. 直接の子のみ
    areas := repo.Areas(ctx, &AreasInput{
        ParentCode: &parentCode,
        DatasetTypes: []string{datasetType},
    })
    
    // 2. 間接的な子も含む（deep）
    areas := repo.Areas(ctx, &AreasInput{
        ParentCode: &parentCode,
        DatasetTypes: []string{datasetType},
        Deep: lo.ToPtr(true),
    })
    
    // 3. データを持つ地域の親も含む（includeParents）
    areas := repo.Areas(ctx, &AreasInput{
        ParentCode: &parentCode,
        DatasetTypes: []string{datasetType},
        IncludeParents: lo.ToPtr(true),
    })
}
```

#### クエリ設計のベストプラクティス

1. **建築物データの検索**
   ```graphql
   # ❌ 悪い例：結果が空になる
   {
     areas(input: {
       parentCode: "04",
       datasetTypes: ["bldg"]
     }) {
       code
       name
     }
   }
   
   # ✅ 良い例：includeParentsまたはdeepを使用
   {
     areas(input: {
       parentCode: "04",
       datasetTypes: ["bldg"],
       includeParents: true
     }) {
       code
       name
     }
   }
   ```

2. **都市計画データの検索**
   ```graphql
   # ✅ 市レベルのデータなので、そのまま検索可能
   {
     areas(input: {
       parentCode: "04",
       datasetTypes: ["urf"]
     }) {
       code
       name
     }
   }
   ```

3. **複数データタイプの検索**
   ```graphql
   # OR条件で検索
   {
     areas(input: {
       datasetTypes: ["bldg", "urf"],
       includeParents: true
     }) {
       code
       name
       type  # PREFECTURE/CITY/WARD
     }
   }
   ```

4. **階層を意識した表示**
   ```graphql
   # 都道府県 → 市 → 区の階層で表示
   {
     area(code: "04") {
       name
       children {  # 市一覧
         name
         code
         ... on City {
           wards {  # 区一覧（政令指定都市のみ）
             name
             code
           }
         }
       }
     }
   }
   ```

## 9. トラブルシューティング

### 9.1 よくある問題

#### 空の結果が返される場合

**症状**: `areas`クエリが空の配列を返す

**原因と対処法**:

1. **データセットタイプと地域階層の不一致**
   ```graphql
   # 問題のあるクエリ
   {
     areas(input: {
       parentCode: "04",
       datasetTypes: ["bldg"]
     }) {
       code
       name
     }
   }
   ```
   
   対処法：`includeParents: true`または`deep: true`を追加
   
2. **データがまだ存在しない地域**
   - キャッシュファイルを確認して、該当地域のデータが存在するか確認
   - CMSでデータが公開されているか確認

3. **フィルタ条件が厳しすぎる**
   ```graphql
   # 条件を緩和して確認
   {
     areas(input: {
       # parentCodeを外す
       datasetTypes: ["bldg"]
     }) {
       code
       name
     }
   }
   ```

#### データが更新されない場合

**症状**: CMSで更新したデータがAPIに反映されない

**原因と対処法**:

1. **キャッシュの問題**
   ```bash
   # キャッシュディレクトリの確認
   ls -la cache/
   
   # キャッシュファイルのタイムスタンプ確認
   ls -la cache/repo_plateau-2024.json
   
   # キャッシュを削除して再起動
   rm -rf cache/*
   ```

2. **Webhookの設定ミス**
   - CMS側のWebhook設定を確認
   - サーバーログでWebhook受信を確認
   ```bash
   grep "webhook" server.log
   ```

3. **部分的な更新失敗**
   - warnings.txtを確認
   ```bash
   cat cache/warnings.txt | grep "failed"
   ```

#### パフォーマンスの問題

**症状**: クエリの応答が遅い

**原因と対処法**:

1. **大量データの取得**
   ```graphql
   # 悪い例：全データを取得
   {
     datasets {
       id
       name
       items {
         url
         layers
       }
     }
   }
   
   # 良い例：必要な地域に絞る
   {
     datasets(input: {
       areaCodes: ["04100"]
     }) {
       id
       name
     }
   }
   ```

2. **N+1クエリ問題**
   - 必要なフィールドのみを要求
   - バッチ処理を活用

3. **メモリ不足**
   - サーバーのメモリ使用量を確認
   - 必要に応じてメモリを増設

### 9.2 デバッグ方法

#### キャッシュファイルの確認
```bash
# メインキャッシュの構造確認
jq '.areas | keys' cache/repo_plateau-2024.json

# 特定地域のデータセット確認
jq '.datasets.plateau[] | select(.cityCode == "04100")' cache/repo_plateau-2024.json

# データセットタイプの一覧
jq '.datasetTypes | keys' cache/repo_plateau-2024.json
```

#### warnings.txtの読み方
```
# 一般的な警告パターン
plateau xxx: city not found: yyy
→ PLATEAUデータxxxが参照する市yyyが見つからない

related zzz: unknown type: aaa
→ 関連データzzzのタイプaaaが未定義

failed to get feature items (bldg): ...
→ 建築物データの取得に失敗
```

#### ログの活用
```bash
# エラーログの確認
grep ERROR server.log | tail -20

# CMS関連のログ
grep "cms" server.log | grep -v "success"

# GraphQLエラー
grep "graphql error" server.log
```

### 9.3 問題の切り分け

#### CMS側の問題
確認項目：
1. CMSにログインして、データが存在するか確認
2. データのステータスが「公開」になっているか
3. 必須フィールドが入力されているか
4. アセットURLが有効か

テスト方法：
```bash
# CMSのAPIを直接叩いてテスト
curl -H "Authorization: Bearer $CMS_TOKEN" \
  "$CMS_URL/api/projects/$PROJECT_ID/models/plateau-city/items"
```

#### 変換処理の問題
確認項目：
1. warnings.txtに変換エラーがないか
2. 期待されるデータ構造と一致しているか

デバッグコードの追加：
```go
// conv.goに一時的にログを追加
log.Printf("Converting city: %+v", cityItem)
```

#### API側の問題
GraphQL Playgroundでのテスト：
```graphql
# 最小限のクエリでテスト
{
  __schema {
    types {
      name
    }
  }
}

# 特定のノードを直接取得
{
  node(id: "city:04100") {
    ... on City {
      name
      code
    }
  }
}
```

## 10. 開発ガイド

### 10.1 新しいデータタイプの追加

#### 手順
1. **CMSモデルの定義**
   ```go
   // cms_model.goに追加
   type NewDataItem struct {
       ID       string `json:"id,omitempty" cms:"id"`
       City     string `json:"city,omitempty" cms:"city,reference"`
       Name     string `json:"name,omitempty" cms:"name,text"`
       Data     string `json:"data,omitempty" cms:"data,asset"`
       // ...
   }
   ```

2. **変換関数の実装**
   ```go
   // conv_dataset_new.goを作成
   func convertNewData(
       items []*NewDataItem,
       dt plateauapi.DatasetType,
       ic *internalContext,
   ) ([]plateauapi.Dataset, []string) {
       // 変換ロジック
   }
   ```

3. **GraphQLスキーマの更新**
   ```graphql
   type NewDatasetType implements DatasetType & Node {
       id: ID!
       code: String!
       name: String!
       # ...
   }
   ```

4. **テストの追加**
   ```go
   func TestConvertNewData(t *testing.T) {
       // テストケース
   }
   ```

### 10.2 フィルタリング条件の拡張

新しいフィルタ条件を追加する場合：

1. **GraphQLスキーマの更新**
   ```graphql
   input AreasInput {
       # 既存フィールド...
       
       # 新しいフィルタ条件
       hasSpecificFeature: Boolean
   }
   ```

2. **フィルタ関数の更新**
   ```go
   func filterArea(a Area, input AreasInput, ...) bool {
       // 既存の条件...
       
       // 新しい条件
       if input.HasSpecificFeature != nil && *input.HasSpecificFeature {
           if !hasSpecificFeature(a) {
               return false
           }
       }
       
       return true
   }
   ```

### 10.3 パフォーマンスの最適化

#### インデックスの追加
```go
// 新しいインデックスマップ
type InMemoryRepo struct {
    // 既存フィールド...
    
    // 新しいインデックス
    areasByName map[string][]AreaCode
}

// インデックスの構築
func buildAreasByNameIndex(areas Areas) map[string][]AreaCode {
    // 実装
}
```

#### クエリの最適化
```go
// バッチ処理の活用
func (r *Repo) BatchGetNodes(ctx context.Context, ids []ID) ([]Node, error) {
    // 一度に複数のノードを取得
}
```

### 10.4 テストの書き方

#### ユニットテスト
```go
func TestAreasQuery(t *testing.T) {
    // テストデータの準備
    repo := NewInMemoryRepo(&InMemoryRepoContext{
        Areas: Areas{
            AreaTypePrefecture: []Area{
                NewPrefecture("04", "宮城県", nil),
            },
        },
    })
    
    // クエリの実行
    areas, err := repo.Areas(context.Background(), &AreasInput{
        ParentCode: lo.ToPtr(AreaCode("04")),
    })
    
    // 検証
    assert.NoError(t, err)
    assert.Len(t, areas, 1)
}
```

#### 統合テスト
```go
func TestEndToEndDataFlow(t *testing.T) {
    // CMSモックの準備
    // データ取得・変換・API提供までの一連の流れをテスト
}
```

## 付録

### A. 用語集

| 用語 | 説明 |
|------|------|
| PLATEAU | 国土交通省が推進する3D都市モデルプロジェクト |
| CityGML | 都市の3Dモデルを記述するための国際標準フォーマット |
| 3D Tiles | 大規模な3Dデータをウェブで効率的に配信するための形式 |
| 政令指定都市 | 人口50万人以上で政令により指定された市（20市） |
| 地域コード | 都道府県（2桁）・市区町村（5桁）を表す行政コード |
| 最詳細レベル | データセットが実際に存在する最も詳細な地域階層 |
| インメモリリポジトリ | 全データをメモリ上に保持する高速データストア |

### B. 設定ファイルのリファレンス

#### 環境変数
```bash
# サーバー設定
PORT=8080
PLATEAU_ENV=development

# CMS設定
CMS_BASE_URL=https://cms.example.com
CMS_TOKEN=your-token-here
CMS_PROJECTS=plateau-2023,plateau-2024

# キャッシュ設定
CACHE_DIR=./cache
CACHE_ENABLED=true

# デバッグ設定
DEBUG=false
DUMP_CACHE=true
```

### C. GraphQL APIリファレンス

主要なクエリの一覧：
- `node(id: ID!)`: IDによるオブジェクト取得
- `nodes(ids: [ID!]!)`: 複数IDによる一括取得
- `area(code: AreaCode!)`: 地域コードによる地域取得
- `areas(input: AreasInput)`: 地域の検索
- `datasets(input: DatasetsInput)`: データセットの検索
- `datasetTypes(input: DatasetTypesInput)`: データセットタイプの検索
- `plateauSpecs`: PLATEAU仕様の一覧
- `years`: 対応年度の一覧

### D. よくある質問（FAQ）

**Q: 建築物データを持つ市を検索するには？**
A: `includeParents: true`を使用してください。

**Q: 特定の年度のデータのみを取得するには？**
A: `DatasetsInput`の`year`フィールドを指定してください。

**Q: データの更新頻度は？**
A: Webhookによりリアルタイム更新されますが、キャッシュにより最大数分の遅延があります。

**Q: 大量のデータを効率的に取得するには？**
A: 必要なフィールドのみを指定し、地域コードで絞り込んでください。