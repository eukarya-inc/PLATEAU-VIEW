# PLATEAU VIEW E2E テスト開発ガイド

このドキュメントは、PLATEAU VIEWのE2Eテスト開発時のベストプラクティスをまとめたものです。

## クイックスタート

```bash
# 全テストを実行
npm test

# スモークテスト（WebKit・高速）
npm run test:smoke

# 特定のテストファイルを実行
npm test -- tests/toolbar/menu.spec.ts

# UIモード（インタラクティブ）
npm test -- --ui

# デバッグモード
npm test -- --debug
```

## 開発フロー

### 1. テストケースを理解する

仕様を確認し、何をテストするのかを明確にする。

### 2. Playwright MCPで動作確認（重要！）

```shell
# Playwright MCPを有効化
claude mcp add-json playwright '{"name":"playwright","command":"npx","args":["@playwright/mcp@latest"]}'
```

実装前に必ず実際の動作を確認：

```typescript
// ブラウザを開く
await mcp__playwright__browser_navigate({ url: 'https://plateauview.mlit.go.jp' });

// 要素を確認
await mcp__playwright__browser_snapshot();

// 操作を実行
await mcp__playwright__browser_click({ element: 'メニューボタン', ref: 'e45' });
```

### 3. テストを実装する

Page Object Modelパターンを使用して実装。

## テストの高速化

### ページインスタンスの共有

複数のテストで同じページを共有することで、初期化コストを削減できます。

```typescript
import { init } from '../../utils';

test.describe.configure({ mode: 'serial' });

test.describe('テストスイート', () => {
  let page: Page;

  test.beforeAll(async ({ browser }, testInfo) => {
    // ヘルパー関数でページを初期化（動画記録も自動設定）
    const { page: newPage } = await init(browser, testInfo);
    page = newPage;
  });

  test.beforeEach(async () => {
    // 各テスト前に状態をリセット
  });
});
```

### パフォーマンスデータ

**実測値（4テストケースの場合）:**
- 変更前（beforeEach）: 平均108秒
- 変更後（beforeAll）: 平均78秒
- **約27%の高速化**

### 推奠されるページ共有の範囲

| 範囲 | テスト数 | 推奨度 | 理由 |
|-----|---------|--------|------|
| 最適 | 5-8 | ★★★ | 初期化コストを効率的に分散 |
| 許容 | 10-20 | ★★☆ | パフォーマンス向上あり |
| 非推奨 | 20以上 | ★☆☆ | メモリ蓄積によるパフォーマンス低下 |

**WebKitの場合**: より多くのテスト（10-15）でも効率的に動作

### beforeAllでの動画記録（ワークアラウンド）

Playwrightの既知の問題により、`beforeAll`では動画設定が自動適用されません。`utils/index.ts`の`init`関数がこの問題を解決します。

**関連Issues:**
- [#11644](https://github.com/microsoft/playwright/issues/11644)
- [#14813](https://github.com/microsoft/playwright/issues/14813)
- [#33720](https://github.com/microsoft/playwright/issues/33720)

## ブラウザ別の対応

### パフォーマンス比較

| ブラウザ | 速度/テスト | 5テストの時間 | 推奨用途 |
|---------|------------|--------------|----------|
| WebKit | 1-4秒 | 約10-20秒 | スモークテスト、開発中の確認 |
| Firefox | 2-11秒 | 約30-50秒 | バランスの良いテスト |
| Chromium | 13-37秒 | 約60-180秒 | 詳細な互換性確認 |

### Chromium/Firefoxのクリック問題

並列実行時にクリック操作がタイムアウトする問題があります。

**解決策**: JavaScriptクリックを使用（BasePageに実装済み）

```typescript
async clickElement(selector: string) {
  if (this.browserName === 'chromium' || this.browserName === 'firefox') {
    await this.page.evaluate((sel) => {
      const element = document.querySelector(sel) as HTMLElement;
      if (element) element.click();
    }, selector);
  } else {
    await this.page.locator(selector).click();
  }
}
```

## Page Object Model

### 基本構造

```
pages/
├── BasePage.ts      # 共通機能（クリック処理、待機処理）
├── ToolbarPage.ts   # ツールバー関連の共通機能
├── MenuPage.ts      # メニュー固有の機能
└── index.ts         # エクスポート
```

### 実装例

```typescript
// MenuPage.ts
export class MenuPage extends ToolbarPage {
  async openMenu() {
    if (!(await this.isMenuOpen())) {
      await this.clickMenuButton();
      await this.page.waitForTimeout(1000);
    }
  }
}
```

## セレクターのベストプラクティス

### MUIコンポーネントの識別

```typescript
// メニュー（role="menu"を持つ）
const menu = page.locator('.MuiModal-root.MuiMenu-root');

// モーダル（メニュー以外）
const modal = page.locator('.MuiModal-root:not(.MuiMenu-root)');
```

### data-name属性の活用

```typescript
// PLATEAU VIEWのメニューアイテム
const menuItem = page.locator('li[role="menuitem"][data-name="my-data"]');
```

### 待機処理

```typescript
// ❌ 避ける: networkidleは使わない（タイルデータの読み込みが続くため）
await page.waitForLoadState('networkidle');

// ✅ 推奨: 特定要素の表示を待つ
await page.waitForSelector('button[aria-label="メインメニュー"]', {
  state: 'visible',
  timeout: 30000
});
```

## デバッグ

### 動画記録

```typescript
// playwright.config.ts
use: {
  video: 'retain-on-failure',  // 失敗時のみ保存
  // video: 'on',              // 常に保存（デバッグ時）
}
```

### スクリーンショット

```typescript
await page.screenshot({ path: 'debug.png' });
```

### ヘッドレスモード無効化

```bash
npm test -- --headed
```

## よくある問題と解決策

### 1. メニュー/モーダルの開閉

- ESCキーを優先的に使用
- 閉じない場合はBackdropクリック
- 適切な待機時間（500-1000ms）を設ける

### 2. テキストの重複

複数の同じテキストがある場合は親要素から絞り込む：

```typescript
const helpModal = page.locator('.MuiModal-root').last();
await expect(helpModal.getByText('ヘルプ', { exact: true })).toBeVisible();
```

### 3. スモークテストのタグ

```typescript
test.describe('基本機能 @smoke', () => {
  test('重要なテスト @critical', async () => {
    // CI/CDで最優先で実行されるテスト
  });
});
```

## 重要な教訓

1. **実装前に必ずPlaywright MCPで動作確認**
2. **ブラウザ別の挙動の違いを考慮**
3. **適切な待機処理を入れる**
4. **Page Object Modelで保守性を高める**
5. **実際のDOM構造を確認してからセレクターを決める**
