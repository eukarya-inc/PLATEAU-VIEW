import { test, expect, type Page } from '@playwright/test';
import { init, waitFor, waitForCesiumStable } from '../utils';
import { ToolbarPage, CanvasPage, AttributesPage } from '../pages';

test.describe.configure({ mode: 'serial' });

test.describe('建築物の属性表示 @smoke', () => {
  let page: Page;
  let toolbarPage: ToolbarPage;
  let canvasPage: CanvasPage;
  let attributesPage: AttributesPage;

  test.beforeAll(async ({ browser }, testInfo) => {
    const result = await init(browser, testInfo);
    page = result.page;
    toolbarPage = new ToolbarPage(page, browser);
    canvasPage = new CanvasPage(page, browser);
    attributesPage = new AttributesPage(page, browser);

    // PLATEAU VIEWを開く
    await toolbarPage.goto();

    // ページの準備を待つ
    await toolbarPage.waitForPageReady();

    // Cesiumが安定するまで待機（建築物の表示完了を含む）
    await waitForCesiumStable(page, testInfo);
  });

  test('選択モードで建築物をクリックすると属性が表示される', async () => {
    // 選択モードに切り替える
    await toolbarPage.switchToSelectMode();

    // 建築物をクリック（画面中央より少し下）
    await canvasPage.clickAt(0.5, 0.6);

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
    await toolbarPage.switchToMoveMode();

    // 建築物をクリック（画面中央より少し下）
    await canvasPage.clickAt(0.5, 0.6);

    // 属性パネルが表示されないことを確認
    await attributesPage.waitForAttributePanelToHide();
  });
});
