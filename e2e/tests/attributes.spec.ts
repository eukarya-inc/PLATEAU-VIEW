import { test, expect, type Page } from '@playwright/test';
import { init, waitFor } from '../utils';
import { AttributesPage } from '../pages';

test.describe.configure({ mode: 'serial' });

test.describe('建築物の属性表示 @smoke', () => {
  let page: Page;
  let attributesPage: AttributesPage;

  test.beforeAll(async ({ browser }, testInfo) => {
    const result = await init(browser, testInfo);
    page = result.page;
    attributesPage = new AttributesPage(page, browser.browserType().name());

    // PLATEAU VIEWを開く
    await attributesPage.goto();
    
    // ページの準備を待つ
    await attributesPage.waitForPageReady();

    // 3Dデータの読み込み待ち（GPUなし環境では自動的に長くなる）
    await waitFor(page, 3000);
  });

  test.afterAll(async () => {
    await page.close();
  });

  test('選択モードで建築物をクリックすると属性が表示される', async () => {
    // 選択モードに切り替える
    await attributesPage.switchToSelectMode();

    // 建築物をクリック
    await attributesPage.clickBuilding();

    // UIの反応を待つ
    await waitFor(page, 1000);

    // 属性パネルが表示されることを確認
    await attributesPage.waitForAttributePanel();

    // 建築物の属性が表示されているか確認
    const hasAttributes = await attributesPage.hasBuildingAttributes();
    expect(hasAttributes).toBeTruthy();
  });

  test('属性パネルを閉じることができる', async () => {
    // パネルが表示されていることを確認
    await attributesPage.waitForAttributePanel();

    // パネルを閉じる
    await attributesPage.closeAttributePanel();

    // パネルが閉じられたことを確認
    await attributesPage.waitForAttributePanelToHide();
  });

  test('移動モードでは建築物クリックしても属性が表示されない', async () => {
    // 移動モードに切り替える
    await attributesPage.switchToMoveMode();

    // 建築物をクリック
    await attributesPage.clickBuilding();

    // UIの反応を待つ
    await waitFor(page, 1000);

    // 属性パネルが表示されないことを確認
    await attributesPage.waitForAttributePanelToHide();
  });
});
