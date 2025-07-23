import { test, expect } from '@playwright/test';
import { MenuPage } from '../../pages';

// 各テストを並列実行（独立）
test.describe.configure({ mode: 'parallel' });

test.describe('Toolbar - メニュー', () => {
  let menuPage: MenuPage;

  test.beforeEach(async ({ page, browserName }) => {
    // Page Objectを初期化
    menuPage = new MenuPage(page, browserName);

    // ページに遷移して初期化を待つ
    await menuPage.goto();
    await menuPage.waitForPageReady();
  });

  test('001: ロゴクリック', async ({ page }) => {
    // メニューを開く
    await menuPage.openMenu();

    // メニューが開いていることを確認
    await expect(await menuPage.isMenuOpen()).toBe(true);

    // メニューアイテムが表示されていることを確認
    const menuItems = await menuPage.verifyAllMenuItems();
    expect(menuItems['Myデータ']).toBe(true);
    expect(menuItems['ヘルプ']).toBe(true);
    expect(menuItems['フィードバック']).toBe(true);
    expect(menuItems['3D都市モデルダウンロード']).toBe(true);

    // UIを隠すアイテムも確認
    await expect(page.getByRole('menuitem', { name: 'UIを隠す' })).toBeVisible();

    // メニューを閉じる
    await menuPage.closeMenu();
    await expect(await menuPage.isMenuOpen()).toBe(false);
  });

  test('002: Myデータ', async ({ page }) => {
    // Myデータを開く
    await menuPage.openMyData();

    // Myデータモーダルが表示されることを確認
    await expect(await menuPage.isMyDataModalOpen()).toBe(true);

    // Myデータダイアログのタイトルを確認
    await expect(page.getByText('Myデータ', { exact: true })).toBeVisible();

    // タブが表示されることを確認
    await expect(page.getByRole('tab', { name: 'ローカルのデータから追加' })).toBeVisible();
    await expect(page.getByRole('tab', { name: 'Webから追加' })).toBeVisible();

    // ファイルアップロードエリアが表示されることを確認
    await expect(page.getByText('ここをクリックしてファイルを選択するか')).toBeVisible();

    // ダイアログを閉じる
    await menuPage.closeModal();
    await expect(await menuPage.isMyDataModalOpen()).toBe(false);
  });

  test('003: ヘルプ', async ({ page }) => {
    // メニューを開く
    await menuPage.openMenu();

    // ヘルプメニューアイテムが表示されていることを確認
    await expect(await menuPage.hasMenuItem('ヘルプ')).toBe(true);

    // ヘルプメニューをクリック
    await menuPage.clickMenuItem('ヘルプ');

    // ヘルプモーダルが表示されることを確認
    const helpModal = page.locator('.MuiModal-root').last();
    await expect(helpModal).toBeVisible();

    // ヘルプダイアログのタイトルを確認（モーダル内のタイトルのみを探す）
    await expect(helpModal.getByText('ヘルプ', { exact: true })).toBeVisible();

    // ヘルプダイアログのタブが表示されることを確認
    await expect(page.getByRole('tab', { name: 'UIを理解する' })).toBeVisible();
    await expect(page.getByRole('tab', { name: 'マップ操作' })).toBeVisible();
    await expect(page.getByRole('tab', { name: 'レイヤー' })).toBeVisible();
    await expect(page.getByRole('tab', { name: 'インスペクター' })).toBeVisible();

    // ダイアログを閉じる
    await menuPage.closeModal();
    await expect(helpModal).not.toBeVisible();
  });

  test('004: フィードバック', async () => {
    // メニューを開く
    await menuPage.openMenu();

    // フィードバックメニューアイテムが表示されていることを確認
    await expect(await menuPage.hasMenuItem('フィードバック')).toBe(true);

    // 画面表示の確認のみ行い、実際にはクリックしない（国交省に連絡が行くため）
  });
});
