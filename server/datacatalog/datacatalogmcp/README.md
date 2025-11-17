# PLATEAU Data Catalog MCP Server

このパッケージは、PLATEAU（プラトー）の3D都市モデルデータカタログをAIクライアントから簡単にアクセスできるようにするMCP（Model Context Protocol）サーバーを提供します。

> [!WARNING]
> **試験的な提供 - 利用上の注意**
>
> このMCPサーバーは試験的な提供です。以下の点にご注意ください：
>
> - **動作保証はありません** - 予告なく仕様変更や停止が行われる可能性があります
> - **SLA（サービスレベル保証）はありません** - 可用性やパフォーマンスの保証は一切行いません
> - **本番環境での利用は推奨しません** - 検証・評価目的での利用を想定しています
> - **サポートは限定的です** - 問い合わせへの対応は保証されません
>
> 本サービスの利用により生じたいかなる損害についても、提供者は一切の責任を負いません。

## 概要

PLATEAU Data Catalog MCP Serverは、PLATEAUの都市モデルデータへのアクセスを提供するHTTPベースのMCPサーバーです。以下の情報にアクセスできます：

- **地域情報**: 都道府県、市区町村などの地域データ
- **データセット情報**: 建物、道路、土地利用などの3D都市モデルデータセット
- **データセット種類**: 利用可能なデータセットの種類と分類
- **メタデータ**: PLATEAU全体の統計情報や仕様バージョン

## AIクライアントでの設定方法

### Claude Desktop での設定

Claude Desktop の設定ファイル (`claude_desktop_config.json`) に以下を追加します：

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "plateau-catalog": {
      "url": "https://api.plateauview.mlit.go.jp/datacatalog/mcp",
      "transport": {
        "type": "http"
      }
    }
  }
}
```

設定後、Claude Desktopを再起動すると、以下の6つのツールが利用可能になります：

- `plateau_get_metadata` - PLATEAU全体のメタデータ取得
- `plateau_search_areas` - 地域検索
- `plateau_get_area` - 地域詳細取得
- `plateau_search_datasets` - データセット検索
- `plateau_get_dataset` - データセット詳細取得
- `plateau_list_dataset_types` - データセット種類一覧

### その他のMCP対応AIクライアント

HTTP MCPをサポートする他のAIクライアントでも、同様にサーバーURLを設定することで利用できます：

```
https://api.plateauview.mlit.go.jp/datacatalog/mcp
```

## 利用可能なツール

### 1. `plateau_get_metadata`
PLATEAU全体のメタデータを取得します。

**パラメータ**: なし

**レスポンス例**:
```json
{
  "available_years": [2020, 2021, 2022, 2023],
  "plateau_specs": [
    {
      "id": "...",
      "major_version": 3,
      "year": 2023,
      "minor_versions": [...]
    }
  ],
  "total_areas": 150,
  "total_datasets": 1200
}
```

### 2. `plateau_search_areas`
地域（都道府県、市区町村）を検索します。

**パラメータ**:
- `parent_code` (optional): 親地域コード（例: "13" で東京都）
- `dataset_types` (optional): データセット種類コードのリスト
- `categories` (optional): カテゴリ（PLATEAU, RELATED, GENERIC）
- `area_types` (optional): 地域タイプ（PREFECTURE, CITY, WARD）
- `search_text` (optional): 検索文字列
- `include_parents` (optional): 親地域を含めるか
- `include_empty` (optional): データセットがない地域も含めるか
- `deep` (optional): 深い階層まで検索するか

**レスポンス例**:
```json
{
  "areas": [
    {
      "id": "...",
      "type": "CITY",
      "code": "13101",
      "name": "千代田区",
      "parent_id": "...",
      "dataset_count": 15
    }
  ],
  "metadata": {
    "total_count": 50,
    "returned_count": 50,
    "has_more": false,
    "refinement_suggestions": []
  }
}
```

### 3. `plateau_get_area`
特定の地域の詳細情報を取得します。

**パラメータ**:
- `code` (required): 地域コード（例: "13101"）

**レスポンス例**:
```json
{
  "id": "...",
  "type": "CITY",
  "code": "13101",
  "name": "千代田区",
  "parent": {
    "id": "...",
    "code": "13",
    "name": "東京都"
  },
  "children": [],
  "planar_crs_epsg_code": "6677"
}
```

### 4. `plateau_search_datasets`
データセットを検索します。

**パラメータ**:
- `area_codes` (optional): 地域コードのリスト
- `dataset_types` (optional): データセット種類コードのリスト
- `categories` (optional): カテゴリ（PLATEAU, RELATED, GENERIC）
- `plateau_spec` (optional): PLATEAU仕様バージョン
- `year` (optional): 整備年度
- `registration_year` (optional): 登録年度
- `search_text` (optional): 検索文字列
- `shallow` (optional): 詳細情報を省略するか

**レスポンス例**:
```json
{
  "datasets": [
    {
      "id": "...",
      "name": "千代田区_建築物モデル（LOD2、テクスチャなし）",
      "description": "...",
      "type": {
        "code": "bldg",
        "name": "建築物モデル",
        "category": "PLATEAU"
      },
      "area": {
        "prefecture": "東京都",
        "city": "千代田区",
        "ward": null
      },
      "year": 2023,
      "registration_year": 2023,
      "plateau_spec": "第3.0版",
      "item_count": 3
    }
  ],
  "metadata": {
    "total_count": 120,
    "returned_count": 100,
    "has_more": true,
    "refinement_suggestions": [
      "地域コードで絞り込む (area_codes パラメータ)",
      "データセット種類で絞り込む (dataset_types パラメータ)"
    ]
  }
}
```

### 5. `plateau_get_dataset`
特定のデータセットの詳細情報を取得します。

**パラメータ**:
- `id` (required): データセットID

**レスポンス例**:
```json
{
  "id": "...",
  "name": "千代田区_建築物モデル（LOD2、テクスチャなし）",
  "description": "...",
  "type": {
    "code": "bldg",
    "name": "建築物モデル",
    "category": "PLATEAU"
  },
  "area": {
    "prefecture": {
      "id": "...",
      "code": "13",
      "name": "東京都"
    },
    "city": {
      "id": "...",
      "code": "13101",
      "name": "千代田区"
    },
    "ward": null
  },
  "year": 2023,
  "registration_year": 2023,
  "plateau_spec": {
    "name": "第3.0版",
    "version": "3.0"
  },
  "groups": ["建築物"],
  "open_data_url": "https://...",
  "items": [
    {
      "id": "...",
      "name": "LOD2（OP2）",
      "format": "CITYGML",
      "url": "https://...",
      "lod": 2,
      "texture": "NONE",
      "layers": []
    }
  ]
}
```

### 6. `plateau_list_dataset_types`
データセット種類の一覧を取得します。

**パラメータ**:
- `category` (optional): カテゴリ（PLATEAU, RELATED, GENERIC）
- `plateau_spec` (optional): PLATEAU仕様バージョン
- `year` (optional): 対象年度

**レスポンス例**:
```json
{
  "dataset_types": [
    {
      "id": "...",
      "code": "bldg",
      "name": "建築物モデル",
      "category": "PLATEAU",
      "year": 2023,
      "dataset_count": 150
    },
    {
      "id": "...",
      "code": "tran",
      "name": "道路モデル",
      "category": "PLATEAU",
      "year": 2023,
      "dataset_count": 120
    }
  ]
}
```

## 使用例

### 東京都内の市区町村を検索

```json
{
  "tool": "plateau_search_areas",
  "parameters": {
    "parent_code": "13",
    "area_types": ["CITY", "WARD"]
  }
}
```

### 千代田区の建築物モデルを検索

```json
{
  "tool": "plateau_search_datasets",
  "parameters": {
    "area_codes": ["13101"],
    "dataset_types": ["bldg"]
  }
}
```

### 2023年度のPLATEAUデータセットを検索

```json
{
  "tool": "plateau_search_datasets",
  "parameters": {
    "year": 2023,
    "categories": ["PLATEAU"]
  }
}
```

## 制限事項

- 検索結果は最大100件までに制限されています
- 100件を超える場合、`metadata.has_more` が `true` となり、`refinement_suggestions` に絞り込み方法が提示されます
- より詳細な検索を行う場合は、パラメータを追加して絞り込んでください

## 技術仕様

- プロトコル: MCP (Model Context Protocol) v1.0
- トランスポート: HTTP (単一JSONレスポンス、SSE非使用)
- メッセージ形式: JSON-RPC 2.0
- レスポンス形式: JSON
- 認証: なし（公開データのため）
- レート制限: なし

### MCP 仕様準拠

このサーバーは [Model Context Protocol (MCP)](https://spec.modelcontextprotocol.io/) の公式仕様に準拠しています：

- **JSON-RPC 2.0**: すべての通信は JSON-RPC 2.0 メッセージ形式で行われます
- **HTTP トランスポート**: 単一JSONレスポンス形式（`Content-Type: application/json`）
- **SSE非使用**: Server-Sent Eventsは使用せず、通常のHTTPリクエスト/レスポンスで動作
- **自動エンドポイント**: `/` エンドポイントで以下が自動的に提供されます：
  - `tools/list`: 利用可能なツールの一覧取得
  - `tools/call`: ツールの実行
- **ステートレスモード**: セッション管理なしでシンプルに利用可能
- **実装ライブラリ**: [mark3labs/mcp-go](https://github.com/mark3labs/mcp-go) v0.43.0 を使用

## 関連リンク

- [PLATEAU 公式サイト](https://www.mlit.go.jp/plateau/)
- [Model Context Protocol (MCP) 仕様](https://spec.modelcontextprotocol.io/)
