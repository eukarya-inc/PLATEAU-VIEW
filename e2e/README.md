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

## スモークテスト

スモークテストは、アプリケーションの基本的な機能が正常に動作することを確認する軽量なテストセットです。

```bash
# すべてのスモークテストを実行
npm run test:smoke

# 重要度の高いスモークテストのみ実行（Chromiumのみ）
npm run test:smoke:critical

# Chromiumでのみスモークテストを実行
npm run test:smoke:chromium
```

### タグの種類

- `@smoke` - スモークテストとして実行されるテスト
- `@critical` - 最も重要な基本機能のテスト
- `@toolbar` - ツールバー関連のテスト
- `@menu` - メニュー関連のテスト
- `@search` - 検索機能のテスト
- `@navigation` - ナビゲーション機能のテスト
- `@3d` - 3D表示機能のテスト

### カスタムタグでの実行

```bash
# 特定のタグのテストのみ実行
npm test -- --grep @menu

# 複数のタグを組み合わせて実行
npm test -- --grep "@smoke.*@menu"

# 特定のタグを除外して実行
npm test -- --grep-invert @slow
```

## 開発方法

詳細な開発ガイドは[CLAUDE.md](./CLAUDE.md)を参照してください。

## テストコードの生成

Playwrightのコードジェネレータを使用してテストコードを生成できます：

```bash
yarn codegen https://plateauview.mlit.go.jp
```
