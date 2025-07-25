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

# 型チェック
npm run type-check
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
import { init, waitForCesiumStable } from '../../utils';

test.describe.configure({ mode: 'serial' });

test.describe('テストスイート', () => {
  let page: Page;

  test.beforeAll(async ({ browser }, testInfo) => {
    // ヘルパー関数でページを初期化（動画記録も自動設定）
    const { page: newPage } = await init(browser, testInfo);
    page = newPage;

    // Cesiumが安定するまで待機（3Dコンテンツのロード完了）
    await waitForCesiumStable(page, testInfo);
  });

  test.beforeEach(async () => {
    // 各テスト前に状態をリセット
  });
});
```

### Cesiumの安定待機

3Dコンテンツが完全にロードされるまで待機する関数を使用：

```typescript
// テスト名が自動的にログに含まれる
await waitForCesiumStable(page, testInfo);
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

## Page Object Model (POM)

pages ディレクトリ内にPOMを定義することで、テストコードの可読性が向上します。

### Page Object設計の考え方

#### 単一ページアプリケーションにおけるPage Object

PLATEAU VIEWのような単一ページアプリケーションでは、Page Objectは「ページ」ではなく「UI領域」や「機能領域」を表現します。これは従来の複数ページアプリケーションとは異なるアプローチです。

#### 責任範囲の決定原則

Page Objectの責任範囲を決める際の思考プロセス：

1. **視覚的境界より論理的境界を重視**
   - UIコンポーネントの見た目の配置ではなく、機能的な関連性で分割
   - 例：選択/移動ボタンは属性パネルと連動するが、ツールバーの機能として扱う

2. **ユーザーの認知モデルに従う**
   - ユーザーが「ツールバー」「メニュー」「3Dビュー」として認識する単位でPage Objectを作成
   - 実装の都合ではなく、ユーザー視点での機能分類を優先

3. **操作の起点で責任を割り当てる**
   - 「どこから操作が始まるか」を基準に責任を決定
   - 結果が別の領域に表示されても、操作の起点となるPage Objectが責任を持つ

#### 抽象度の設計

1. **具体的すぎるメソッドを避ける**
   - `clickBuilding()` → `clickAt(x, y)`
   - テストが意図を明示的に表現できるレベルの抽象度を保つ

2. **コンテキストをPage Objectに埋め込まない**
   - Page Objectはツールを提供し、使い方はテストが決める
   - 「建築物をクリックする」という知識はテスト側に置く

3. **待機処理は適切な場所に**
   - UI要素の表示待機はPage Object内で完結
   - ビジネスロジックの待機（データロード等）はテスト側で明示

### 実装例（Playwright公式ベストプラクティスに準拠）

```typescript
// BasePage.ts - Locatorオブジェクトを使用
export class BasePage {
  readonly page: Page;

  // Locatorプロパティとして定義
  readonly menuButton: Locator;
  readonly searchInput: Locator;

  constructor(page: Page, browser: Browser) {
    this.page = page;
    this.browser = browser;

    // Locatorの初期化
    this.menuButton = page.getByRole('button', { name: 'メインメニュー' });
    this.searchInput = page.getByPlaceholder('データセット、建築物、住所を検索');
  }

  // 動的Locatorを返すメソッド
  button(label: string): Locator {
    return this.page.getByRole('button', { name: label });
  }
}

// AttributesPage.ts - 継承して使用
export class AttributesPage extends BasePage {
  readonly selectButton: Locator;
  readonly canvas: Locator;

  constructor(page: Page, browser: Browser) {
    super(page, browser);

    // ページ固有のLocator
    this.selectButton = page.getByRole('button', { name: '選択' });
    this.canvas = page.locator('canvas').first();
  }

  async switchToSelectMode() {
    await this.selectButton.click();
    await expect(this.selectButton).toHaveAttribute('aria-pressed', 'true');
  }
}
```

### Page Object初期化と使用例

```typescript
// テストファイル内
test.beforeAll(async ({ browser }, testInfo) => {
  const { page } = await init(browser, testInfo);

  // 各UI領域に対応するPage Objectを初期化
  const toolbarPage = new ToolbarPage(page, browser);
  const canvasPage = new CanvasPage(page, browser);
  const attributesPage = new AttributesPage(page, browser);
});

// 使用例：建築物の属性表示テスト
test('選択モードで建築物をクリックすると属性が表示される', async () => {
  // ツールバーで選択モードに切り替え
  await toolbarPage.switchToSelectMode();

  // 3Dビューで建築物をクリック（座標を直接指定）
  await canvasPage.clickAt(0.5, 0.6);

  // 属性パネルの表示を確認
  await attributesPage.waitForAttributePanel();
});
```

## セレクターのベストプラクティス

### MUIコンポーネントの識別

```typescript
// メニュー（role="menu"を持つ）
const menu = page.locator('.MuiModal-root.MuiMenu-root');

// モーダル（メニュー以外）
const modal = page.locator('.MuiModal-root:not(.MuiMenu-root)');
```

### data-testidやdata-name属性の活用

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

環境変数 `VIDEO_MODE=true` で録画

```bash
VIDEO_MODE=true npm test
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
  test('スモークテスト @smoke', async () => {
    // CI/CDで優先的に実行されるテスト
  });
});
```

## 重要な教訓

1. **実装前に必ずPlaywright MCPで動作確認**
2. **ブラウザ別の挙動の違いを考慮**
3. **適切な待機処理を入れる**
4. **Page Object Modelで保守性を高める**
5. **実際のDOM構造を確認してからセレクターを決める**
