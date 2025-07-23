# PLATEAU VIEW E2E テスト開発ガイド

このドキュメントは、PLATEAU VIEWのE2Eテスト開発時のノウハウと注意点をまとめたものです。

## 開発フロー

### 1. テストケースの理解

- まず`test-cases.md`でテストケースを確認
- どのような操作を行い、何を検証するのかを理解する

### 2. Playwright MCPで手動確認（重要！）

実装前に必ずPlaywright MCPを使って実際の動作を確認します：

```typescript
// ブラウザを開く
await mcp__playwright__browser_navigate({ url: 'https://plateauview.mlit.go.jp' });

// スナップショットを取得して要素を確認
await mcp__playwright__browser_snapshot();

// 実際にクリックして動作を確認
await mcp__playwright__browser_click({ element: 'メニューボタン', ref: 'e45' });
```

**なぜ重要か:**
- 実際のセレクターが確認できる
- モーダルなのかダイアログなのか、新しいタブが開くのかが分かる
- エラーや想定外の動作を事前に発見できる

### 3. テストコードの実装
MCPで確認した動作を基にテストコードを実装します。

## ブラウザ別の注意点

### Chromiumの並列実行問題
Chromiumでは並列実行時にクリック操作がタイムアウトする問題があります。

**症状:**
```
TimeoutError: locator.click: Timeout 10000ms exceeded.
Call log:
  - waiting for getByRole('button', { name: 'メインメニュー' })
  - locator resolved to <button...>
  - attempting click action
  - waiting for element to be visible, enabled and stable
  - element is visible, enabled and stable
  - scrolling into view if needed
  - done scrolling
```

**解決策:**
JavaScriptによる直接クリックを使用します：

```typescript
// Chromium用のクリックヘルパー関数
async function clickElement(page: Page, selector: string, browserName: string) {
  if (browserName === 'chromium') {
    await page.evaluate((sel) => {
      const element = document.querySelector(sel) as HTMLElement;
      if (element) element.click();
    }, selector);
  } else {
    await page.locator(selector).click();
  }
}

// 使用例
await clickByRole(page, 'button', 'メインメニュー', browserName);
```

### Firefoxでの問題
Firefoxでも並列実行時にクリック操作がタイムアウトすることがあります。

**症状:**
- 要素は表示されているが、別の要素がクリックを妨げている
- `<div class="css-fmgov4">...</div> intercepts pointer events` というエラー

**解決策:**
Chromiumと同様にJavaScriptクリックを使用します。

### ブラウザ間の互換性対応
```typescript
// BasePage.tsでブラウザごとの処理を統一
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

## 待機処理のベストプラクティス

### 避けるべきこと
```typescript
// ❌ networkidleは使わない（タイルデータをバックグラウンドで読み込むため）
await page.waitForLoadState('networkidle');
```

### 推奨される待機処理
```typescript
// ✅ domcontentloadedを使用
await page.waitForLoadState('domcontentloaded');

// ✅ 特定の要素を待つ
await page.waitForSelector('button[aria-label="メインメニュー"]', {
  state: 'visible',
  timeout: 30000
});

// ✅ 固定の待機時間（UIの初期化完了を待つ）
await page.waitForTimeout(5000);
```

## セレクターの選び方

### MUIコンポーネントの注意点
Material-UIのコンポーネントは独自のクラス構造を持っています：

#### メニューの識別
```typescript
// メニューは MuiMenu-root クラスを持つ
const menu = page.locator('.MuiModal-root.MuiMenu-root');
```

#### モーダルの識別
```typescript
// Myデータモーダルなど、メニュー以外のモーダル
const modal = page.locator('.MuiModal-root:not(.MuiMenu-root)');
```

#### 特殊なメニューアイテム
```typescript
// 3D都市モデルダウンロードは <a> タグだが role="menuitem"
const downloadLink = page.locator('a[role="menuitem"]:has-text("3D都市モデルダウンロード")');
```

### data-name属性の活用
PLATEAU VIEWのメニューアイテムは`data-name`属性を持っています：

```typescript
// data-name属性のマッピング
const dataNameMapping = {
  'Myデータ': 'my-data',
  'ヘルプ': 'help',
  'フィードバック': 'feedback',
  'UIを隠す': 'hide-ui',
  // 注意: '3D都市モデルダウンロード'はdata-name属性を持たない
};

// セレクターの例
const menuItem = page.locator('li[role="menuitem"][data-name="my-data"]');
```

### 複数の同じテキストがある場合
```typescript
// ヘルプモーダル内のタイトルを探す（メニューのヘルプと区別）
const helpModal = page.locator('.MuiModal-root').last();
await expect(helpModal.getByText('ヘルプ', { exact: true })).toBeVisible();
```

## Page Object Model (POM) の実装

### POMの利点
- テストコードの保守性向上
- 共通処理の再利用
- ブラウザ固有の処理を一元管理
- UI変更への対応が容易

### 基本構造
```typescript
// BasePage.ts - 共通機能
export class BasePage {
  protected page: Page;
  protected browserName: string;
  
  // 共通セレクター定義
  protected selectors = {
    menuButton: 'button[aria-label="メインメニュー"]',
    modalRoot: '.MuiModal-root',
    closeButton: 'button[aria-label="close"]',
    menuItem: (dataName: string) => `li[role="menuitem"][data-name="${dataName}"]`,
  };
  
  // ブラウザ別のクリック処理
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
}
```

### ページクラスの実装例
```typescript
// MenuPage.ts - メニュー操作に特化
export class MenuPage extends ToolbarPage {
  // メニュー関連のセレクター
  protected menuSelectors = {
    menu: '[role="menu"]',
    myDataModal: {
      root: '.MuiModal-root:has-text("Myデータ")',
      localTab: 'button[role="tab"]:has-text("ローカルのデータから追加")',
    },
  };
  
  async openMenu() {
    if (!(await this.isMenuOpen())) {
      await this.clickMenuButton();
      await this.page.waitForTimeout(1000);
    }
  }
  
  async closeMenu() {
    if (await this.isMenuOpen()) {
      // ESCキーでメニューを閉じる
      await this.page.keyboard.press('Escape');
      await this.page.waitForTimeout(500);
    }
  }
}
```

### テストでの使用
```typescript
test.describe('Toolbar - メニュー', () => {
  let menuPage: MenuPage;
  
  test.beforeEach(async ({ page, browserName }) => {
    menuPage = new MenuPage(page, browserName);
    await menuPage.goto();
    await menuPage.waitForPageReady();
  });
  
  test('メニューを開く', async () => {
    await menuPage.openMenu();
    expect(await menuPage.isMenuOpen()).toBe(true);
  });
});
```

## デバッグ手法

### 1. 動画記録を有効にする
```typescript
// playwright.config.ts
use: {
  video: 'on',
}
```

### 2. スクリーンショットを撮る
```typescript
await page.screenshot({ path: 'debug.png' });
```

### 3. セレクターの確認
```typescript
const count = await page.locator('button[aria-label="メインメニュー"]').count();
console.log(`Button count: ${count}`);

const isVisible = await page.locator('button').first().isVisible();
console.log(`Button visible: ${isVisible}`);
```

## テスト構成

### 並列実行の設定
```typescript
// 各テストを独立して実行（推奨）
test.describe.configure({ mode: 'parallel' });

// 順次実行（前のテストの状態が影響する場合）
test.describe.configure({ mode: 'serial' });
```

### タイムアウトの設定
```typescript
test.beforeEach(async ({ page }) => {
  // テストごとのタイムアウト
  test.setTimeout(60000);
});
```

## よくある問題と解決策

### 1. メニューが開いているのにモーダルが見つからない
**原因:** セレクターが間違っている、またはタイミングの問題

**解決策:**
- Playwright MCPで実際のDOM構造を確認
- 適切な待機処理を追加

### 2. Chromiumだけテストが失敗する
**原因:** Chromiumの並列実行時の問題

**解決策:**
- JavaScriptクリックヘルパー関数を使用
- ブラウザ別の待機時間を設定

## テスト実行コマンド

```bash
# 全テストを実行
npm test

# 特定のブラウザで実行
npm test -- --project=chromium

# 特定のテストファイルを実行
npm test -- tests/toolbar/menu.spec.ts

# デバッグモード
npm test -- --debug

# UIモード（インタラクティブ）
npm test -- --ui

# ヘッドレスモード無効（ブラウザを表示）
npm test -- --headed
```

## スモークテスト

### 概要
スモークテストは、デプロイ後やCI/CDパイプラインで素早く基本機能を確認するためのテストセットです。

### タグの使い方
```typescript
// テストファイルでタグを定義
test.describe('Toolbar - メニュー @smoke @toolbar', () => {
  // このテストスイートはスモークテストかつツールバーテスト
});

// 個別のテストにもタグを付けられる
test('サイトにアクセスできる @smoke @critical', async ({ page }) => {
  // このテストは重要度の高いスモークテスト
});
```

### スモークテストの実行
```bash
# すべてのスモークテストを実行
npm run test:smoke

# 重要なテストのみChromiumで実行（高速）
npm run test:smoke:critical

# 特定のタグの組み合わせ
npm test -- --grep "@smoke.*@menu"
```

### スモークテストの設計方針
1. **高速実行**: 全体で1-2分以内に完了
2. **基本機能**: アプリケーションの主要な機能のみテスト
3. **安定性**: フレーキーなテストは含めない
4. **独立性**: 各テストは他のテストに依存しない

## 重要な教訓

1. **実装前に必ずPlaywright MCPで動作確認する**
   - DOM構造の確認（特にMUIコンポーネントのクラス構造）
   - 要素のrole属性やdata-name属性の確認
   - モーダル/ダイアログの挙動確認

2. **ブラウザ別の挙動の違いを考慮する**
   - ChromiumとFirefoxはJavaScriptクリックが必要
   - WebKitは通常のクリックで動作
   - 各ブラウザで待機時間の調整が必要な場合がある

3. **適切な待機処理を入れる**
   - `networkidle`は避ける（タイルデータのバックグラウンド読み込みのため）
   - `domcontentloaded`と特定要素の表示待機を組み合わせる
   - モーダル/メニューの開閉には適切な待機時間を設ける

4. **Page Object Modelで保守性を高める**
   - ブラウザ固有の処理はBasePageに集約
   - セレクターは各ページクラスで定義
   - data-name属性のマッピングは一元管理

5. **セレクターは実際のDOM構造を確認してから決める**
   - MUIコンポーネントのクラス構造を理解する
   - 特殊なケース（リンクだがmenuitemなど）に注意
   - 複数の同じテキストがある場合は親要素から絞り込む

6. **メニュー/モーダルの開閉処理**
   - ESCキーを優先的に使用
   - 閉じない場合はBackdropクリックやJavaScript実行
   - 開閉後は適切な待機時間を設ける
