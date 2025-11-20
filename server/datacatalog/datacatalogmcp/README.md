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

### Claude Code での設定

Claude Code では、以下のコマンドで MCP サーバーを追加できます：

```bash
claude mcp add --transport http plateau-catalog https://api.plateauview.mlit.go.jp/datacatalog/mcp
```

**参考**: 設定手順の詳細は [Claude Code MCP 公式ドキュメント](https://docs.anthropic.com/en/docs/claude-code/mcp) をご確認ください。

### Claude Desktop での設定

#### 有料プラン (Pro/Team/Enterprise) の場合

1. Claude Desktop を開く
2. Settings → Integrations を選択
3. 「+ Add Custom Integration」をクリック
4. MCP サーバーの URL を入力：
   ```
   https://api.plateauview.mlit.go.jp/datacatalog/mcp
   ```
5. 設定を保存

#### 無料プランの場合

1. Claude Desktop を開く
2. Settings → Developer タブを選択
3. 「Edit Config」をクリック
4. 設定ファイルに以下を追加：

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

5. ファイルを保存して Claude Desktop を再起動

**参考**: 設定手順の詳細は [Claude Desktop ヘルプセンター](https://support.claude.com/en/articles/10949351-getting-started-with-local-mcp-servers-on-claude-desktop) をご確認ください。

### ChatGPT での設定

ChatGPT (Pro/Team/Enterprise/Edu) では、以下の手順で MCP サーバーを追加できます：

1. ChatGPT の Settings で Developer Mode（開発者モード）を有効化
2. Settings → Connectors → Create を選択
3. 以下の情報を入力：
   - **Connector name**: `PLATEAU Data Catalog`
   - **Description**: `日本の3D都市モデル（PLATEAU）のデータカタログにアクセスできます。地域情報、建築物・道路・土地利用などのデータセット情報を検索・取得できます。`
   - **Connector URL**: `https://api.plateauview.mlit.go.jp/datacatalog/mcp`
4. 設定を保存
5. チャットで「+」ボタン → 「More」→ PLATEAU Data Catalog を選択して利用

**参考**: 設定手順の詳細は [ChatGPT MCP 公式ドキュメント](https://platform.openai.com/docs/mcp) をご確認ください。

### その他のMCP対応AIクライアント

HTTP MCPをサポートする他のAIクライアントでも、同様にサーバーURLを設定することで利用できます：

```
https://api.plateauview.mlit.go.jp/datacatalog/mcp
```

設定後、以下の10個のツールが利用可能になります：

**データカタログツール:**
- `plateau_get_metadata` - PLATEAU全体のメタデータ取得
- `plateau_search_areas` - 地域検索
- `plateau_get_area` - 地域詳細取得
- `plateau_search_datasets` - データセット検索
- `plateau_get_dataset` - データセット詳細取得
- `plateau_list_dataset_types` - データセット種類一覧

**CityGMLツール:**
- `plateau_get_citygml_files` - メッシュコード/空間ID/矩形範囲でCityGMLファイルを検索
- `plateau_citygml_get_attributes` - CityGMLファイルから建物属性情報を取得
- `plateau_citygml_get_features` - 空間IDに交差する地物IDリストを取得
- `plateau_citygml_get_geoid_height` - 緯度経度からジオイド高を取得

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

## CityGML ツール

データカタログツールに加えて、CityGMLファイルから直接属性情報を取得するツールも提供しています。

### 7. `plateau_citygml_get_attributes`
CityGMLファイルから指定した建物IDの属性情報を取得します。

**使い方の流れ:**
1. `plateau_get_citygml_files`でメッシュコードや空間IDを指定してCityGML URLを取得
2. レスポンスの`cities[].files[type][].url`からCityGML URLを取得（typeは'bldg', 'tran'など）
3. そのURLと建物IDをこのツールに渡す

**パラメータ**:
- `url` (required): CityGMLファイルのURL
- `building_ids` (required): 取得する建物IDのリスト
- `skip_code_list` (optional): コードリストの取得をスキップするか（デフォルト: false）

**レスポンス例**:
```json
{
  "attributes": [
    {
      "gml:id": "BLD_12345",
      "bldg:measuredHeight": 30.5,
      "bldg:storeysAboveGround": 10,
      "bldg:usage": "住宅",
      "_bbox": {
        "min": {"lng": 139.7, "lat": 35.6, "alt": 0},
        "max": {"lng": 139.71, "lat": 35.61, "alt": 30.5},
        "center": {"lng": 139.705, "lat": 35.605, "alt": 15.25}
      }
    }
  ]
}
```

### 8. `plateau_citygml_get_features`
CityGMLファイルから指定した空間ID（SpatialID）に交差する地物のIDリストを取得します。

**使い方の流れ:**
1. `plateau_get_citygml_files`でメッシュコードや空間IDを指定してCityGML URLを取得
2. レスポンスの`cities[].files[type][].url`からCityGML URLを取得（typeは'bldg', 'tran'など）
3. そのURLと空間IDをこのツールに渡す
4. 返ってきた地物IDを`plateau_citygml_get_attributes`で使用できる

**パラメータ**:
- `url` (required): CityGMLファイルのURL
- `spatial_ids` (required): 検索する空間IDのリスト（例: ["25/52235/23212/25/0"]）

**レスポンス例**:
```json
{
  "feature_ids": [
    "BLD_12345",
    "BLD_12346",
    "BLD_12347"
  ]
}
```

### 9. `plateau_citygml_get_geoid_height`
指定した緯度経度のジオイド高を取得します。標高の楕円体高と正標高の変換に使用できます。

**パラメータ**:
- `latitude` (required): 緯度（度）
- `longitude` (required): 経度（度）

**レスポンス例**:
```json
{
  "latitude": 35.681236,
  "longitude": 139.767125,
  "geoid_height": 39.456,
  "geoid": "39.456"
}
```

### 10. `plateau_get_citygml_files`
指定した条件でCityGMLファイルを検索します。メッシュコード、空間ID、または矩形範囲で検索できます。

**条件フォーマット**:
- メッシュコード: `m:53393580,53393581` (カンマ区切りで複数指定可)
- 空間ID: `s:15/0/29134/12950,15/0/29134/12951` (カンマ区切りで複数指定可)
- 矩形範囲: `r:139.7,35.6,139.8,35.7` (西経度,南緯度,東経度,北緯度)

**使い方の流れ**:
1. このツールでCityGMLファイルURLを取得
2. 取得したURLを`plateau_citygml_get_features`または`plateau_citygml_get_attributes`で使用

**パラメータ**:
- `condition` (required): 検索条件（例: "m:53393580", "s:15/0/29134/12950", "r:139.7,35.6,139.8,35.7"）

**レスポンス例**:
```json
{
  "cities": [
    {
      "cityCode": "13112",
      "cityName": "世田谷区",
      "year": 2023,
      "registrationYear": 2024,
      "spec": "4.1",
      "url": "https://assets.cms.plateau.reearth.io/assets/.../13112_setagaya-ku_pref_2023_citygml_2_op.zip",
      "files": {
        "bldg": [
          {
            "code": "53393580",
            "maxLod": 2,
            "url": "https://assets.cms.plateau.reearth.io/assets/.../bldg/53393580_bldg_6697_op.gml"
          }
        ],
        "tran": [
          {
            "code": "53393580",
            "maxLod": 2,
            "url": "https://assets.cms.plateau.reearth.io/assets/.../tran/53393580_tran_6697_op.gml"
          }
        ]
      }
    }
  ],
  "featureTypes": {
    "bldg": {
      "name": "建築物モデル"
    },
    "tran": {
      "name": "交通（道路）モデル"
    }
  }
}
```

### CityGML ツールの使用例

#### 特定地域の建物属性を取得

```
1. まず、世田谷区のメッシュコードでCityGML URLを取得:
   plateau_get_citygml_files(condition="m:53393580")

2. レスポンスのcities[0].files.bldg[0].urlから建物のCityGML URLを取得

3. 空間IDで地物を検索:
   plateau_citygml_get_features(url="取得したURL", spatial_ids=["15/0/29134/12950"])

4. 返ってきた建物IDで属性を取得:
   plateau_citygml_get_attributes(url="取得したURL", building_ids=["BLD_12345", "BLD_12346"])
```

#### 空間IDで複数種類のデータを取得

```
1. 空間IDでCityGMLファイルを検索:
   plateau_get_citygml_files(condition="s:15/0/29134/12950")

2. レスポンスから複数の種類のファイルを取得:
   - cities[0].files.bldg[].url: 建築物モデル
   - cities[0].files.tran[].url: 交通（道路）モデル
   - cities[0].files.dem[].url: 地形モデル

3. 各URLから必要な属性を取得
```

#### 矩形範囲で広域検索

```
1. 矩形範囲でCityGMLファイルを検索（東京駅周辺）:
   plateau_get_citygml_files(condition="r:139.7,35.6,139.8,35.7")

2. 複数の市区町村のデータが返ってくる:
   - cities[0]: 千代田区のデータ
   - cities[1]: 中央区のデータ
   ...

3. 必要な市区町村のファイルを選択して属性を取得
```

## 関連リンク

- [PLATEAU 公式サイト](https://www.mlit.go.jp/plateau/)
- [Model Context Protocol (MCP) 仕様](https://spec.modelcontextprotocol.io/)
