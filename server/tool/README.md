# PLATEAU VIEW Server Tools

PLATEAU VIEWサーバーのCMS連携・データ移行ツール集です。

## 概要

このディレクトリには、PLATEAU VIEWのCMSデータ管理に関する以下のツールが含まれています：

- `setup-city-items`: 都市アイテムをCSVから一括登録
- `copy-related`: 関連データセットアイテムをプロジェクト間でコピー
- `upload-assets`: アセットファイルを一括アップロード
- `migrate-v1`: PLATEAU VIEW v1からのデータ移行

## 使い方

サーバーディレクトリから以下のコマンドで実行します：

```bash
cd server
go run . <サブコマンド> [オプション]
```

または、ビルドして実行：

```bash
cd server
go build
./server <サブコマンド> [オプション]
```

### 環境変数の設定

ツール実行時に毎回`-base`や`-token`オプションを指定するのは面倒なので、`.env`ファイルに設定しておくことをおすすめします。

#### .envファイルの作成

`server`ディレクトリに`.env`ファイルを作成し、以下のように設定します：

```bash
# server/.env
REEARTH_PLATEAUVIEW_CMS_BASEURL=https://your-cms-url.com
REEARTH_PLATEAUVIEW_CMS_TOKEN=your-token-here
```

注意: 環境変数名には`REEARTH_PLATEAUVIEW_`プレフィックスが必要です。

#### 簡易設定

プレフィックスを省略した設定も可能です（ただし、正式には`REEARTH_PLATEAUVIEW_`プレフィックス付きが推奨されます）：

```bash
# server/.env（プレフィックスなし）
CMS_BASEURL=https://your-cms-url.com
CMS_TOKEN=your-token-here
CMS_SYSTEMPROJECT=your-system-project-id
```

`.env`ファイルが存在する場合、ツール起動時に自動的に読み込まれます。ログに`config: .env loaded`と表示されれば成功です。

各コマンドで`-base`、`-token`オプションを指定することで、`.env`の設定を上書きできます。

---

## コマンド一覧

### 1. setup-city-items

CSVファイルから都市アイテムをCMSに一括登録します。

#### 使い方

```bash
go run . setup-city-items \
  -project <PROJECT_ID> \
  -system-project <SYSTEM_PROJECT_ID> \
  -file <CSV_FILE_PATH> \
  [-base <CMS_BASE_URL>] \
  [-token <CMS_TOKEN>] \
  [-force] \
  [-offset <NUMBER>] \
  [-limit <NUMBER>] \
  [-dryrun]
```

#### オプション

| オプション | 必須 | 説明 | デフォルト |
|---------|------|------|-----------|
| `-project` | ✓ | CMSプロジェクトID | - |
| `-system-project` | ✓ | CMSシステムプロジェクトID | 環境変数`CMS_SYSTEMPROJECT` |
| `-file` | ✓ | CSVファイルのパス | - |
| `-base` | | CMS Base URL | 環境変数`CMS_BASEURL` |
| `-token` | | CMS認証トークン | 環境変数`CMS_TOKEN` |
| `-force` | | 既存データがあっても強制実行 | false |
| `-offset` | | 処理開始位置（行番号） | 0 |
| `-limit` | | 処理する最大件数 | 0（無制限） |
| `-dryrun` | | 実際には登録せずに動作確認のみ | false |

#### CSVフォーマット

CSVファイルは以下のフォーマットで作成してください：

**ヘッダー行（必須）:**
```csv
city_code,city,city_en,pref,bldg,tran,trk,squr,rwy,wwy,urf,luse,fld,tnm,htd,ifld,rfld,lsld,frn,veg,wtr,dem,brid,tun,cons,ubld,unf,area,gen
```

**データ行:**
```csv
13101,千代田区,chiyoda-ku,東京都,LOD2,LOD3,,,,,LOD1,LOD1,LOD1,LOD1,LOD1,LOD1,LOD1,LOD1,LOD3,LOD2,LOD1,LOD1,LOD2,,,LOD4,,,
```

##### 必須カラム

- `city_code`: 都市コード（5桁）。空の場合は自動生成されます
- `city`: 都市名（日本語）
- `city_en`: 都市名（英語）
- `pref`: 都道府県名

##### 地物タイプカラム（5列目以降）

各地物タイプに対応するカラムに、LOD値を指定します：

- 値が空でない場合、その地物タイプが有効になります
- 値の例: `LOD1`, `LOD2`, `LOD3`, `LOD4`
- 空の場合、その地物タイプはスキップされます

主な地物タイプ：
- `bldg`: 建築物
- `tran`: 道路
- `trk`: 軌道
- `squr`: 広場
- `rwy`: 鉄道
- `wwy`: 航路
- `urf`: 都市計画決定情報
- `luse`: 土地利用
- `fld`: 洪水浸水想定区域
- `tnm`: 津波浸水想定
- `htd`: 高潮浸水想定区域
- `ifld`: 内水浸水想定区域
- `rfld`: 土砂災害警戒区域
- `lsld`: 土砂災害警戒区域
- `frn`: 都市設備
- `veg`: 植生
- `wtr`: 水部
- `dem`: 地形
- `brid`: 橋梁
- `tun`: トンネル
- `cons`: 建設
- `ubld`: 地下街
- `unf`: 地下施設
- `area`: エリア
- `gen`: 汎用

#### 実行例

```bash
# ドライラン（実際には登録しない）
go run . setup-city-items \
  -project 01234567-89ab-cdef-0123-456789abcdef \
  -system-project fedcba98-7654-3210-fedc-ba9876543210 \
  -file /path/to/cities.csv \
  -dryrun

# 本番実行（.envにCMS_SYSTEMPROJECTを設定している場合）
go run . setup-city-items \
  -project 01234567-89ab-cdef-0123-456789abcdef \
  -file /path/to/cities.csv

# 100件目から10件だけ処理
go run . setup-city-items \
  -project 01234567-89ab-cdef-0123-456789abcdef \
  -system-project fedcba98-7654-3210-fedc-ba9876543210 \
  -file /path/to/cities.csv \
  -offset 100 \
  -limit 10
```

---

### 2. copy-related

関連データセットアイテムを、あるプロジェクトから別のプロジェクトにコピーします。

#### 使い方

```bash
go run . copy-related \
  -src <SOURCE_PROJECT_ID> \
  -target <TARGET_PROJECT_ID> \
  [-base <CMS_BASE_URL>] \
  [-token <CMS_TOKEN>] \
  [-offset <NUMBER>] \
  [-limit <NUMBER>] \
  [-cache <CACHE_PATH>] \
  [-force] \
  [-dryrun]
```

#### オプション

| オプション | 必須 | 説明 | デフォルト |
|---------|------|------|-----------|
| `-src` | ✓ | コピー元プロジェクトID | - |
| `-target` | ✓ | コピー先プロジェクトID | - |
| `-base` | | CMS Base URL | 環境変数`CMS_BASEURL` |
| `-token` | | CMS認証トークン | 環境変数`CMS_TOKEN` |
| `-offset` | | 処理開始位置 | 0 |
| `-limit` | | 処理する最大件数 | 0（無制限） |
| `-cache` | | キャッシュディレクトリのパス | - |
| `-force` | | 既存データがあっても強制実行 | false |
| `-dryrun` | | 実際にはコピーせずに動作確認のみ | false |

#### 実行例

```bash
# ドライラン
go run . copy-related \
  -src 01234567-89ab-cdef-0123-456789abcdef \
  -target 76543210-fedc-ba98-7654-3210fedcba98 \
  -dryrun

# 本番実行（キャッシュ使用）
go run . copy-related \
  -src 01234567-89ab-cdef-0123-456789abcdef \
  -target 76543210-fedc-ba98-7654-3210fedcba98 \
  -cache ./cache
```

---

### 3. upload-assets

指定ディレクトリ内のZIPファイルをCMSに一括アップロードします。

#### 使い方

```bash
go run . upload-assets \
  -project <PROJECT_ID> \
  -target <DIRECTORY_PATH> \
  [-base <CMS_BASE_URL>] \
  [-token <CMS_TOKEN>]
```

#### オプション

| オプション | 必須 | 説明 | デフォルト |
|---------|------|------|-----------|
| `-project` | ✓ | CMSプロジェクトID | - |
| `-target` | ✓ | アップロード対象ディレクトリ | - |
| `-base` | | CMS Base URL | 環境変数`CMS_BASEURL` |
| `-token` | | CMS認証トークン | 環境変数`CMS_TOKEN` |

#### 実行例

```bash
go run . upload-assets \
  -project 01234567-89ab-cdef-0123-456789abcdef \
  -target /path/to/assets/
```

**注意:** `.zip`拡張子のファイルのみがアップロードされます。

---

### 4. migrate-v1

PLATEAU VIEW v1からv3へのデータ移行用ツールです。指定されたリストファイルに基づいてファイルをダウンロードし、ZIPアーカイブを作成します。

#### 使い方

```bash
go run . migrate-v1 \
  -list <LIST_FILE_PATH> \
  -base <BASE_URL> \
  -output <OUTPUT_DIRECTORY> \
  [-prefix <PREFIX>] \
  [-offset <NUMBER>] \
  [-wetrun]
```

#### オプション

| オプション | 必須 | 説明 | デフォルト |
|---------|------|------|-----------|
| `-list` | ✓ | ファイルリストのパス | - |
| `-base` | ✓ | ダウンロード元のベースURL | - |
| `-output` | ✓ | 出力先ディレクトリ | - |
| `-prefix` | | フィルタリング用のプレフィックス | - |
| `-offset` | | 処理開始位置 | 0 |
| `-wetrun` | | 実際にダウンロードを実行（指定しない場合はドライラン） | false |

#### 実行例

```bash
# ドライラン
go run . migrate-v1 \
  -list /path/to/list.txt \
  -base https://old-server.example.com \
  -output ./output

# 本番実行
go run . migrate-v1 \
  -list /path/to/list.txt \
  -base https://old-server.example.com \
  -output ./output \
  -wetrun
```

---

## トラブルシューティング

### エラー: "CMS base URL is required"

環境変数`CMS_BASEURL`が設定されていないか、`-base`オプションが指定されていません。

**解決方法:**
```bash
# .envファイルに設定（推奨）
cat >> server/.env << EOF
REEARTH_PLATEAUVIEW_CMS_BASEURL=https://your-cms-url.com
REEARTH_PLATEAUVIEW_CMS_TOKEN=your-token-here
REEARTH_PLATEAUVIEW_CMS_SYSTEMPROJECT=your-system-project-id
EOF

# または、コマンドで直接指定
go run . setup-city-items \
  -base https://your-cms-url.com \
  -token your-token-here \
  -system-project your-system-project-id \
  ...
```

### エラー: "CMS system project ID is required"

システムプロジェクトIDが設定されていません。

**解決方法:**
`.env`ファイルに`REEARTH_PLATEAUVIEW_CMS_SYSTEMPROJECT`を追加するか、`-system-project`オプションで指定してください：
```bash
# .envファイルに追加
echo "REEARTH_PLATEAUVIEW_CMS_SYSTEMPROJECT=your-system-project-id" >> server/.env

# または、コマンドで指定
go run . setup-city-items -system-project your-system-project-id ...
```

### エラー: "city items already exist"

既に都市アイテムが存在しています。

**解決方法:**
既存データを上書きする場合は`-force`オプションを追加してください：
```bash
go run . setup-city-items -force ...
```

### エラー: "invalid header"

CSVファイルのヘッダーが正しくありません。

**解決方法:**
CSVファイルのヘッダー行に以下の必須カラムがあることを確認してください：
- `pref`
- `city`
- `city_en`

### CSVファイルのエンコーディングエラー

CSVファイルがUTF-8でエンコードされていることを確認してください。

**解決方法（macOS/Linux）:**
```bash
# ファイルのエンコーディングを確認
file -I your-file.csv

# Shift-JISからUTF-8に変換
iconv -f SHIFT-JIS -t UTF-8 input.csv > output.csv
```

---

## 開発者向け情報

### コードの場所

- メインエントリーポイント: `tool.go`
- 各ツールの実装:
  - `setup-city-items.go` → `../cmsintegration/cmsintsetup/setup_cities.go`
  - `copy-related-items.go` → `../cmsintegration/cmsintsetup/copy_related.go`
  - `upload-assets.go`
  - `migrate-v1.go`

### テスト

```bash
cd server
go test ./cmsintegration/cmsintsetup/...
```

### 新しいツールの追加方法

1. `tool/`ディレクトリに新しいファイルを作成（例: `new-tool.go`）
2. 関数を実装：
   ```go
   func newTool(conf *Config, args []string) error {
       // 実装
       return nil
   }
   ```
3. `tool.go`の`Main`関数に新しいケースを追加：
   ```go
   case "new-tool":
       err = newTool(conf, args)
   ```

---

## ライセンス

APACHE-2.0
