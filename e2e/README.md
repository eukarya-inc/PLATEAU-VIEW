# PLATEAU VIEW E2E Tests

PLATEAU VIEWのEnd-to-Endテストです。

## セットアップ

```bash
cd e2e
npm i
npm run test:install
```

## テスト実行

```bash
# 全てのテストを実行
npm test

# ヘッドレスモードで実行（ブラウザを表示）
npm run test:headed

# UIモードで実行（インタラクティブなテストランナー）
npm run test:ui

# デバッグモードで実行
npm run test:debug

# 特定のテストファイルを実行
npm test -- tests/toolbar/menu.spec.ts

# レポートを表示
npm run report
```

## テストコードの生成

Playwrightのコードジェネレータを使用してテストコードを生成できます：

```bash
yarn codegen https://plateauview.mlit.go.jp
```
