# GraphQL API リファレンス

エンドポイント:

```
https://api.plateauview.mlit.go.jp/datacatalog/graphql
```

ブラウザで開くと組み込みの **GraphiQL** が起動し、対話的にクエリを試せる。

## 特徴

- 必要なフィールドだけ 1 リクエストで取得
- 都道府県・自治体・年度などでのフィルタリング
- スキーマは [plateauapi (GitHub)](https://github.com/Project-PLATEAU/PLATEAU-VIEW-5.0/tree/main/server/datacatalog/plateauapi) のソースから生成
- スキーマリファレンスは <https://docs.plateauview.mlit.go.jp/api/graphql/schema/>

## 基本クエリ例

### 自治体コードからデータセット一覧

```graphql
query {
  area(code: "13101") {
    id
    type
    datasets {
      id
      name
      items {
        id
        name
        url
      }
    }
  }
}
```

```bash
curl -X POST https://api.plateauview.mlit.go.jp/datacatalog/graphql \
  -H 'Content-Type: application/json' \
  --data '{"query":"{ area(code: \"13101\") { datasets { name items { url } } } }"}'
```

### CityGML データセット検索

```graphql
query {
  citygmlDatasets(input: { prefectureCodes: ["13"] }) {
    id
    cityCode
    url
    year
    featureTypes
  }
}
```

## スキーマを調べる方法

1. **公式スキーマリファレンス**: <https://docs.plateauview.mlit.go.jp/api/graphql/schema/>
2. **GraphiQL の introspection**: エンドポイントをブラウザで開いてドキュメント探索
3. **生 SDL**: <https://github.com/Project-PLATEAU/PLATEAU-VIEW-5.0/tree/main/server/datacatalog/plateauapi>

## 制約

- スキーマ・レスポンスは予告なく変更される
- 複雑すぎるクエリは速度低下防止のため制限される
- レスポンスサイズは数 MB 以上になることがある
