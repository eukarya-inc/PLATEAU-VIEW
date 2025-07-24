import { test, expect } from '@playwright/test';
import { BasePage, DEFAULT_URL } from '../pages/BasePage';
import { MenuPage } from '../pages';
import { init } from '../utils';

// スモークテストは全ブラウザで順次実行
test.describe.configure({ mode: 'serial' });

test.describe('スモークテスト @smoke', () => {
  let basePage: BasePage;
  let menuPage: MenuPage;

  test.beforeAll(async ({ browser, browserName }, testInfo) => {
    // ヘルパー関数を使用してコンテキストとページを作成
    const { page } = await init(browser, testInfo);

    // Page Objectを初期化
    basePage = new BasePage(page, browserName);
    menuPage = new MenuPage(page, browserName);

    // ページに遷移して初期化を待つ（1回だけ）
    await basePage.goto();
    await basePage.waitForPageReady();
  });

  test('サイトにアクセスできる @smoke', async () => {
    // ページタイトルの確認
    await expect(basePage.page).toHaveTitle(/PLATEAU VIEW/);

    // 3D表示エリアが存在することを確認
    const canvas = basePage.page.locator('canvas');
    await expect(canvas).toBeVisible();
  });

  test('メインメニューが動作する @smoke @menu', async () => {
    // メニューボタンが表示されている
    const menuButton = basePage.page.getByRole('button', { name: 'メインメニュー' });
    await expect(menuButton).toBeVisible();

    // メニューを開く
    await menuPage.openMenu();
    await expect(await menuPage.isMenuOpen()).toBe(true);

    // 基本的なメニューアイテムが表示されている
    await expect(await menuPage.hasMenuItem('Myデータ')).toBe(true);
    await expect(await menuPage.hasMenuItem('ヘルプ')).toBe(true);

    // メニューを閉じる
    await menuPage.closeMenu();
    await expect(await menuPage.isMenuOpen()).toBe(false);
  });

  test('検索機能が利用できる @smoke @search', async () => {
    // 検索バーが表示されている
    const searchInput = basePage.page.getByPlaceholder('データセット、建築物、住所を検索');
    await expect(searchInput).toBeVisible();

    // 検索バーに入力できる（Chromium対応のクリック処理を使用）
    await basePage.clickSearchInput();
    await searchInput.fill('東京');
    await expect(searchInput).toHaveValue('東京');

    // 検索バーをクリア（Chromium対応のクリア処理を使用）
    await basePage.clearSearchInput();
    await expect(searchInput).toHaveValue('');
  });

  test('ナビゲーションコントロールが表示される @smoke @navigation', async () => {
    // 基本的なナビゲーションボタンが表示されている
    const zoomInButton = basePage.page.getByRole('button', { name: '拡大' });
    const zoomOutButton = basePage.page.getByRole('button', { name: '縮小' });
    const compassButton = basePage.page.getByRole('button', { name: 'コンパス' });

    await expect(zoomInButton).toBeVisible();
    await expect(zoomOutButton).toBeVisible();
    await expect(compassButton).toBeVisible();
  });

  test('3Dビューが正常に読み込まれる @smoke @3d', async () => {
    // Cesiumのcanvasが存在する
    const canvas = basePage.page.locator('canvas');
    await expect(canvas).toBeVisible();

    // WebGL contextが作成されている
    const hasWebGL = await basePage.page.evaluate(() => {
      const canvas = document.querySelector('canvas');
      if (!canvas) return false;
      const gl = canvas.getContext('webgl') || canvas.getContext('webgl2');
      return gl !== null;
    });

    expect(hasWebGL).toBe(true);
  });
});
