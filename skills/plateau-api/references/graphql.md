# GraphQL API

エンドポイント: `https://api.plateauview.mlit.go.jp/datacatalog/graphql`

## スキーマ取得

introspection で動的に取得する。スキーマは予告なく変更されるので、コード生成等の前に都度取得すること。

最小（型名と種別のみ）:

```bash
curl -fsSL -X POST https://api.plateauview.mlit.go.jp/datacatalog/graphql \
  -H 'Content-Type: application/json' \
  -d '{"query":"{__schema{types{name kind}}}"}'
```

特定の型のフィールド:

```bash
curl -fsSL -X POST https://api.plateauview.mlit.go.jp/datacatalog/graphql \
  -H 'Content-Type: application/json' \
  -d '{"query":"query($n:String!){__type(name:$n){name fields{name type{name kind ofType{name kind}}}}}","variables":{"n":"Area"}}'
```

完全な introspection は SKILL.md のクエリを参照。

## クエリパターン

### 自治体コードからデータセット一覧

```bash
curl -fsSL -X POST https://api.plateauview.mlit.go.jp/datacatalog/graphql \
  -H 'Content-Type: application/json' \
  -d '{"query":"{ area(code: \"13101\") { id type datasets { id name items { id name url } } } }"}'
```

### 都道府県で CityGML データセットを絞る

```bash
curl -fsSL -X POST https://api.plateauview.mlit.go.jp/datacatalog/graphql \
  -H 'Content-Type: application/json' \
  -d '{"query":"{ citygmlDatasets(input: { prefectureCodes: [\"13\"] }) { id cityCode url fileSize year featureTypes } }"}'
```

## 主要フィールド

- `fileSize`: `Int`、null あり、CMS 上の zip のバイト数（`DatasetItem` 系（`PlateauDatasetItem` / `GenericDatasetItem` / `RelatedDatasetItem`）にも同名フィールドあり）
- `RelatedDatasetItem.originalFileSize`: 変換前データのサイズ

## 制約

- スキーマ・レスポンスは予告なく変更される
- 複雑すぎるクエリは速度低下防止のため制限される
- レスポンスサイズは数 MB 以上になることがある
