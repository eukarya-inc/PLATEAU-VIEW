import { test, expect } from '@playwright/test';
import { MenuPage } from '../../pages';

const url = 'https://plateauview.mlit.go.jp/';

// スモークテストは全ブラウザで順次実行
test.describe.configure({ mode: 'serial' });

test.describe('基本的なスモークテスト @smoke @critical', () => {
  test('サイトにアクセスできる', async ({ page }) => {
    await page.goto(url);

    // ページタイトルの確認
    await expect(page).toHaveTitle(/PLATEAU VIEW/);

    // 3D表示エリアが存在することを確認
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible();
  });

  test('メインメニューが動作する @smoke @menu', async ({ page, browserName }) => {
    const menuPage = new MenuPage(page, browserName);

    await menuPage.goto();
    await menuPage.waitForPageReady();

    // メニューボタンが表示されている
    const menuButton = page.getByRole('button', { name: 'メインメニュー' });
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

  test('検索機能が利用できる @smoke @search', async ({ page }) => {
    await page.goto(url);
    await page.waitForLoadState('domcontentloaded');
    await page.waitForTimeout(3000);

    // 検索バーが表示されている
    const searchInput = page.getByPlaceholder('データセット、建築物、住所を検索');
    await expect(searchInput).toBeVisible();

    // 検索バーに入力できる
    await searchInput.click();
    await searchInput.fill('東京');
    await expect(searchInput).toHaveValue('東京');
  });

  test('ナビゲーションコントロールが表示される @smoke @navigation', async ({ page }) => {
    await page.goto(url);
    await page.waitForLoadState('domcontentloaded');
    await page.waitForTimeout(3000);

    // 基本的なナビゲーションボタンが表示されている
    const zoomInButton = page.getByRole('button', { name: '拡大' });
    const zoomOutButton = page.getByRole('button', { name: '縮小' });
    const compassButton = page.getByRole('button', { name: 'コンパス' });

    await expect(zoomInButton).toBeVisible();
    await expect(zoomOutButton).toBeVisible();
    await expect(compassButton).toBeVisible();
  });

  test('3Dビューが正常に読み込まれる @smoke @3d', async ({ page }) => {
    await page.goto(url);
    await page.waitForLoadState('domcontentloaded');

    // Cesiumのcanvasが存在する
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible();

    // WebGL contextが作成されている
    const hasWebGL = await page.evaluate(() => {
      const canvas = document.querySelector('canvas');
      if (!canvas) return false;
      const gl = canvas.getContext('webgl') || canvas.getContext('webgl2');
      return gl !== null;
    });

    expect(hasWebGL).toBe(true);
  });
});
