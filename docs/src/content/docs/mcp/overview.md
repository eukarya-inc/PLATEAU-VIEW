---
title: PLATEAU MCP Server
description: AI クライアント向け Model Context Protocol サーバーの利用方法とツール一覧
---

PLATEAU MCP Server は、PLATEAU の 3D 都市モデルに関する情報を AI クライアントから簡単にアクセスできるようにする MCP（Model Context Protocol）サーバーを提供します。

:::caution[試験的な提供 - 利用上の注意]
- **動作保証・SLA はありません** - 可用性やパフォーマンスの保証は一切行いません
- **予告なく変更されることがあります** - ツールの追加・削除・変更、レスポンス形式の変更などが事前告知なく行われることがあります
- **本番環境での利用は推奨しません** - 検証・評価目的での利用を想定しています
- **サポートは限定的です** - 問い合わせへの対応は保証されません

本サービスの利用により生じたいかなる損害についても、提供者は一切の責任を負いません。
:::

## 概要

PLATEAU MCP Server は、PLATEAU の都市モデルデータおよび仕様書へのアクセスを提供する HTTP ベースの MCP サーバーです。以下の情報にアクセスできます。

### データカタログ

- **地域情報**: 都道府県、市区町村などの地域データ
- **データセット情報**: 建物、道路、土地利用などの 3D 都市モデルデータセット
- **データセット種類**: 利用可能なデータセットの種類と分類
- **メタデータ**: PLATEAU 全体の統計情報や仕様バージョン
- **CityGML データ**: 空間 ID・メッシュコードによる CityGML ファイルの検索と属性取得

### 仕様書

- **3D 都市モデル標準製品仕様書**: PLATEAU の 3D 都市モデルに関する標準仕様
- **3D 都市モデル標準作業手順書**: 3D 都市モデル作成の標準手順

## AI クライアントでの設定方法

:::note
以下のセットアップ手順は各 AI クライアントの仕様変更により変わる可能性があります。最新の情報は各公式ドキュメントをご確認ください。
:::

### Claude Desktop での設定

#### 有料プラン (Pro / Business / Enterprise) の場合

1. Claude Desktop を開く
2. **Settings → Integrations** を選択
3. 「**+ Add Custom Integration**」をクリック
4. 以下の情報を入力
   - **名前**: `PLATEAU MCP`
   - **URL**: `https://api.plateauview.mlit.go.jp/mcp`
   - **詳細設定 → 認証（OAuth）**: 空欄のまま
5. 設定を保存

#### 組織プラン（Enterprise / Business）で管理者が設定する場合

1. Claude の管理コンソールにログイン
2. **管理者設定 → コネクタ** を選択
3. 「カスタムコネクタを追加」をクリック
4. 上記と同じ情報を入力して保存

#### 無料プランの場合

Claude Desktop 無料版は HTTP MCP に対応していないため、HTTP-to-Stdio アダプタ CLI を使用する必要があります。

1. Claude Desktop を開く
2. **Settings → Developer** タブを選択
3. 「**Edit Config**」をクリック
4. 設定ファイルに以下を追加

```json
{
  "mcpServers": {
    "plateau": {
      "command": "npx",
      "args": ["-y", "@pyroprompts/mcp-stdio-to-streamable-http-adapter"],
      "env": {
        "URI": "https://api.plateauview.mlit.go.jp/mcp",
        "MCP_NAME": "plateau"
      }
    }
  }
}
```

5. ファイルを保存して Claude Desktop を再起動

:::tip[Node.js のセットアップが必要]
この方法では `npx` コマンドを使用するため、事前に Node.js をインストールする必要があります。[Node.js 公式サイト](https://nodejs.org/) から LTS 版をダウンロードしてインストールしてください。
:::

**参考**:

- [Claude Desktop ヘルプセンター: ローカル MCP サーバー](https://support.claude.com/en/articles/10949351-getting-started-with-local-mcp-servers-on-claude-desktop)
- [@pyroprompts/mcp-stdio-to-streamable-http-adapter](https://www.npmjs.com/package/@pyroprompts/mcp-stdio-to-streamable-http-adapter)

### ChatGPT での設定

ChatGPT (Pro / Team / Enterprise / Edu) では、以下の手順で MCP サーバーを追加できます。

1. ChatGPT の **Settings** で Developer Mode を有効化
2. **Settings → Apps** を選択
3. 「アプリを作成する」をクリック
4. 以下の情報を入力
   - **名前**: `PLATEAU MCP`
   - **URL**: `https://api.plateauview.mlit.go.jp/mcp`
   - **認証**: `認証なし` を選択
5. 「カスタム MCP サーバーのリスク警告」で「理解したうえで続行」にチェック
6. 設定を保存
7. チャットで「+」ボタン → 「More」→ PLATEAU MCP を選択して利用

参考: [ChatGPT MCP 公式ドキュメント](https://platform.openai.com/docs/mcp)

### Claude Code での設定

```bash
claude mcp add --transport http plateau https://api.plateauview.mlit.go.jp/mcp
```

参考: [Claude Code MCP 公式ドキュメント](https://docs.anthropic.com/en/docs/claude-code/mcp)

### その他の MCP 対応 AI クライアント

HTTP MCP をサポートする他の AI クライアントでも、同様にサーバー URL を設定することで利用できます。

```
https://api.plateauview.mlit.go.jp/mcp
```

## 各ツールの説明

### 仕様書ツール

#### 1. `plateau_spec_outline`

仕様書の目次（アウトライン）を取得します。

**パラメータ**:

- `document_type` (optional): `standard`（標準製品仕様書、デフォルト）または `procedure`（標準作業手順書）
- `depth` (optional): 取得する階層の深さ（1=章のみ、2=章+節、3=より深く）。デフォルト: 2
- `chapter` (optional): 特定の章のみ取得（例: `toc4` でデータ構造の章）
- `format` (optional): `markdown`（デフォルト）または `json`

#### 2. `plateau_spec_read`

特定の節の内容を取得します。デフォルトで子ページの内容も含めて取得します。

**パラメータ**:

- `path` (required): 読み込むパス（例: `/plateaudocument/toc1`、`/plateaudocument/toc4`）
- `document_type` (optional): `standard`（デフォルト）または `procedure`
- `single_page` (optional): `true` の場合、子ページを含めず指定したページのみ取得。デフォルト: `false`
- `include_images` (optional): `true` の場合、base64 エンコードされた画像をマークダウンに含める。デフォルト: `false`

**特徴**:

- 子ページを含む全内容を**並列で高速取得**
- 出力が長すぎる場合は**自動的にトランケート**され、より細かい節ごとに取得するためのヒントが表示される

### データカタログツール

#### 3. `plateau_get_metadata`

PLATEAU 全体のメタデータを取得します。

**パラメータ**: なし

**レスポンス例**:

```json
{
  "available_years": [2020, 2021, 2022, 2023],
  "plateau_specs": [
    { "id": "...", "major_version": 3, "year": 2023, "minor_versions": [...] }
  ],
  "total_areas": 150,
  "total_datasets": 1200
}
```

#### 4. `plateau_search_areas`

地域（都道府県、市区町村）を検索します。

**パラメータ**:

- `parent_code` (optional): 親地域コード（例: `13` で東京都）
- `dataset_types` (optional): データセット種類コードのリスト
- `categories` (optional): カテゴリ（`PLATEAU`, `RELATED`, `GENERIC`）
- `area_types` (optional): 地域タイプ（`PREFECTURE`, `CITY`, `WARD`）
- `search_text` (optional): 検索文字列
- `include_parents` (optional): 親地域を含めるか
- `include_empty` (optional): データセットがない地域も含めるか
- `deep` (optional): 深い階層まで検索するか

#### 5. `plateau_get_area`

特定の地域の詳細情報を取得します。

**パラメータ**:

- `code` (required): 地域コード（例: `13101`）

#### 6. `plateau_search_datasets`

データセットを検索します。

**パラメータ**:

- `area_codes` (optional): 地域コードのリスト
- `dataset_types` (optional): データセット種類コードのリスト
- `categories` (optional): カテゴリ
- `plateau_spec` (optional): PLATEAU 仕様バージョン
- `year` (optional): 整備年度
- `registration_year` (optional): 登録年度
- `search_text` (optional): 検索文字列
- `shallow` (optional): 詳細情報を省略するか

#### 7. `plateau_get_dataset`

特定のデータセットの詳細情報を取得します。

**パラメータ**:

- `id` (required): データセット ID

#### 8. `plateau_list_dataset_types`

データセット種類の一覧を取得します。

**パラメータ**:

- `category` (optional): カテゴリ
- `plateau_spec` (optional): PLATEAU 仕様バージョン
- `year` (optional): 対象年度

### CityGML ツール

データカタログツールに加えて、CityGML ファイルから直接属性情報を取得するツールも提供しています。

#### 9. `plateau_citygml_get_attributes`

CityGML ファイルから指定した建物 ID の属性情報を取得します。

**使い方の流れ**:

1. `plateau_get_citygml_files` でメッシュコードや空間 ID を指定して CityGML URL を取得
2. レスポンスの `cities[].files[type][].url` から CityGML URL を取得
3. その URL と建物 ID をこのツールに渡す

**パラメータ**:

- `url` (required): CityGML ファイルの URL
- `building_ids` (required): 取得する建物 ID のリスト
- `skip_code_list` (optional): コードリストの取得をスキップするか（デフォルト: `false`）

#### 10. `plateau_citygml_get_features`

CityGML ファイルから指定した空間 ID（SpatialID）に交差する地物の ID リストを取得します。

**パラメータ**:

- `url` (required): CityGML ファイルの URL
- `spatial_ids` (required): 検索する空間 ID のリスト（例: `["25/52235/23212/25/0"]`）

#### 11. `plateau_citygml_get_geoid_height`

指定した緯度経度のジオイド高を取得します。日本のジオイド 2011 に基づきます。標高の楕円体高と正標高の変換に使用できます。

**パラメータ**:

- `latitude` (required): 緯度（度）
- `longitude` (required): 経度（度）

#### 12. `plateau_get_citygml_files`

指定した条件で CityGML ファイルを検索します。メッシュコード、空間 ID、または矩形範囲で検索できます。

**条件フォーマット**:

- メッシュコード: `m:53393580,53393581`（カンマ区切りで複数指定可）
- 空間 ID: `s:15/0/29134/12950,15/0/29134/12951`
- 矩形範囲: `r:139.7,35.6,139.8,35.7`（西経度, 南緯度, 東経度, 北緯度）

**主な地物型**:

- `bldg`: 建築物モデル
- `tran`: 交通（道路）モデル
- `luse`: 土地利用モデル
- `dem`: 地形モデル
- `fld`: 洪水浸水想定区域モデル
- `lsld`: 土砂災害警戒区域モデル
- `urf`: 都市計画決定情報モデル

利用可能な全ての地物型は `plateau_list_dataset_types` ツールで取得できます。

**パラメータ**:

- `condition` (required): 検索条件
- `feature_types` (optional): 取得する地物型のリスト（例: `["bldg", "tran"]`）

### ヘルパーツール

#### 13. `plateau_explain_spatial_id`

空間 ID（Spatial ID）の仕様と使い方を解説するマークダウンを返します。

空間 ID は 3 次元空間を一意に識別するための規格で、`{z}/{f}/{x}/{y}` 形式で表現されます。返される解説には以下が含まれます。

- 空間 ID のフォーマットと各要素の意味
- ズームレベルごとの解像度一覧
- 座標計算式
- PLATEAU ツールでの使い方
- 参考リンク

**パラメータ**: なし

## 制限事項

- 検索結果は最大 100 件までに制限されています
- 100 件を超える場合、`metadata.has_more` が `true` となり、`refinement_suggestions` に絞り込み方法が提示されます
- より詳細な検索を行う場合は、パラメータを追加して絞り込んでください

## 技術仕様

このサーバーは [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) の公式仕様に準拠しています。

- プロトコル: [MCP 2025-06-18](https://modelcontextprotocol.io/specification/2025-06-18)
- トランスポート: HTTP（単一 JSON レスポンス形式、SSE 非使用）
- メッセージ形式: JSON-RPC 2.0
- 認証: なし（公開データのため）
- 実装ライブラリ: [mark3labs/mcp-go](https://github.com/mark3labs/mcp-go)

## 関連リンク

- [PLATEAU 公式サイト](https://www.mlit.go.jp/plateau/)
- [3D 都市モデル標準製品仕様書](https://www.mlit.go.jp/plateaudocument/)
- [3D 都市モデル標準作業手順書](https://www.mlit.go.jp/plateaudocument02/)
- [Model Context Protocol (MCP) 仕様](https://modelcontextprotocol.io/specification)
