import { test, expect, type Page } from '@playwright/test';
import { init, waitForCesiumStable } from '../utils';
import { captureMapArea, compareBlackRatio } from '../utils/imageCompare';

test.describe.configure({ mode: 'serial' });

// 画像比較のヘルパー関数
async function performImageComparison(
  beforeImagePath: string,
  afterImagePath: string,
  expectedIncrease: boolean,
  testName: string
) {
  const comparison = await compareBlackRatio(beforeImagePath, afterImagePath);
  
  console.log(`${testName}:`);
  console.log(`  黒成分率の変化: ${(comparison.difference * 100).toFixed(2)}%`);
  console.log(`  黒成分が${expectedIncrease ? '増加' : '減少'}: ${expectedIncrease ? comparison.increased : !comparison.increased}`);
  
  // アサーション
  if (expectedIncrease) {
    expect(comparison.increased).toBe(true);
    expect(comparison.difference).toBeGreaterThan(0.1);
  } else {
    expect(comparison.increased).toBe(false);
    expect(comparison.difference).toBeLessThan(-0.1);
  }
  
  return comparison;
}

// メニューを閉じるヘルパー関数
async function closeMapMenu(page: Page, browserName: string) {
  const mapMenu = page.locator('div[role="tooltip"]');
  const mapButton = page.getByRole('button', { name: '地図' });
  
  if (browserName === 'chromium') {
    // Chromeの場合は特別な処理
    // まずESCキーを試す
    await page.keyboard.press('Escape');
    await page.waitForTimeout(500);
    
    // それでも閉じない場合は、地図以外の場所をクリック
    if (await mapMenu.isVisible()) {
      // ヘッダー部分をクリック（メニューより上の安全な場所）
      await page.mouse.click(640, 30);
      await page.waitForTimeout(500);
    }
  } else {
    // WebKit/Firefoxは地図ボタンで閉じる
    await mapButton.click();
    await page.waitForTimeout(500);
    
    // まだ開いている場合の追加処理
    if (await mapMenu.isVisible()) {
      await page.mouse.click(100, 100);
      await page.waitForTimeout(500);
    }
  }
  
  await expect(mapMenu).not.toBeVisible();
}

test.describe('地図メニュー @map', () => {
  let page: Page;
  let browserName: string;

  test.beforeAll(async ({ browser }, testInfo) => {
    const result = await init(browser, testInfo);
    page = result.page;
    browserName = browser.browserType().name();

    await page.goto('https://plateauview.mlit.go.jp');
    await waitForCesiumStable(page, testInfo);
  });

  test.beforeEach(async () => {
    // 各テスト前に地図メニューが閉じていることを確認
    const mapMenu = page.locator('div[role="tooltip"]');
    if (await mapMenu.isVisible()) {
      // 地図ボタンをクリックしてメニューを閉じる
      const mapButton = page.getByRole('button', { name: '地図' });
      await mapButton.click();
      await page.waitForTimeout(500);
    }
  });

  test('地図アイコンをクリックするとメニューが開く', async () => {
    const mapButton = page.getByRole('button', { name: '地図' });
    await mapButton.click();

    const mapMenu = page.locator('div[role="tooltip"]');
    await expect(mapMenu).toBeVisible();

    await expect(page.getByRole('menuitem', { name: '白地図' })).toBeVisible();
    await expect(page.getByRole('menuitem', { name: '黒地図' })).toBeVisible();
    await expect(page.getByRole('menuitem', { name: '色付き地図' })).toBeVisible();
    await expect(page.getByRole('menuitem', { name: '衛星写真' })).toBeVisible();
    await expect(page.getByRole('menuitem', { name: '標高' })).toBeVisible();

    await expect(page.getByRole('checkbox', { name: '地下を隠す' })).toBeVisible();
    await expect(page.getByRole('checkbox', { name: '地下に入る' })).toBeVisible();
    await expect(page.getByRole('checkbox', { name: '都道府県、市区町村' })).toBeVisible();
    await expect(page.getByRole('checkbox', { name: '道路' })).toBeVisible();
    
    // テスト終了時にメニューを閉じる
    await closeMapMenu(page, browserName);
  });

  test('黒地図を選択すると地図のタイルが黒に切り替わる @smoke', async () => {
    // 変更前の白地図の状態をキャプチャ
    const beforeBlackMapImage = await captureMapArea(page, 'white-map-before-black.png');
    
    const mapButton = page.getByRole('button', { name: '地図' });
    await mapButton.click();

    const mapMenu = page.locator('div[role="tooltip"]');
    await expect(mapMenu).toBeVisible({ timeout: 10000 });
    
    // メニューアイテムが表示されるのを待つ
    await page.waitForTimeout(1000);

    // 黒地図オプションを探す（より具体的なセレクター）
    const blackMapItem = page.locator('[role="menuitem"]').filter({ hasText: '黒地図' }).first();
    await expect(blackMapItem).toBeVisible({ timeout: 5000 });
    await blackMapItem.click();

    // Chromeではメニューが自動的に閉じない場合がある
    if (browserName === 'chromium') {
      await page.waitForTimeout(500);
      if (await mapMenu.isVisible()) {
        // ESCキーまたは外側をクリックしてメニューを閉じる
        await page.keyboard.press('Escape');
        await page.waitForTimeout(500);
        
        if (await mapMenu.isVisible()) {
          await page.mouse.click(640, 30);
          await page.waitForTimeout(500);
        }
      }
    }

    await expect(mapMenu).not.toBeVisible();

    // 地図が切り替わるのを待つ
    await page.waitForTimeout(3000);

    // 黒地図に切り替わった後の状態をキャプチャして比較
    const afterBlackMapImage = await captureMapArea(page, 'black-map-after.png');
    await performImageComparison(beforeBlackMapImage, afterBlackMapImage, true, '白地図→黒地図の変更');

    // 再度地図メニューを開いて、黒地図が選択されていることを確認
    await mapButton.click();
    await expect(mapMenu).toBeVisible();

    const blackMapItemAfter = page.getByRole('menuitem', { name: /黒地図/ });
    const blackMapCheckIcon = blackMapItemAfter.locator('img').first();
    await expect(blackMapCheckIcon).toBeVisible();

    // メニューを閉じる
    await closeMapMenu(page, browserName);
  });

  test('白地図に戻すことができる', async () => {
    // 変更前の黒地図の状態をキャプチャ
    const beforeWhiteMapImage = await captureMapArea(page, 'black-map-before-white.png');
    
    const mapButton = page.getByRole('button', { name: '地図' });
    await mapButton.click();

    const mapMenu = page.locator('div[role="tooltip"]');
    await expect(mapMenu).toBeVisible();

    const whiteMapItem = page.getByRole('menuitem', { name: /白地図/ });
    await whiteMapItem.click();

    // Chromeではメニューが自動的に閉じない場合がある
    if (browserName === 'chromium') {
      await page.waitForTimeout(500);
      if (await mapMenu.isVisible()) {
        await page.keyboard.press('Escape');
        await page.waitForTimeout(500);
        
        if (await mapMenu.isVisible()) {
          await page.mouse.click(640, 30);
          await page.waitForTimeout(500);
        }
      }
    }

    await expect(mapMenu).not.toBeVisible();

    // 地図が切り替わるのを待つ
    await page.waitForTimeout(3000);

    // 白地図に切り替わった後の状態をキャプチャして比較
    const afterWhiteMapImage = await captureMapArea(page, 'white-map-after.png');
    await performImageComparison(beforeWhiteMapImage, afterWhiteMapImage, false, '黒地図→白地図の変更');

    // 再度地図メニューを開いて、白地図が選択されていることを確認
    await mapButton.click();
    await expect(mapMenu).toBeVisible();

    const whiteMapItemAfter = page.getByRole('menuitem', { name: /白地図/ });
    const whiteMapCheckIcon = whiteMapItemAfter.locator('img').first();
    await expect(whiteMapCheckIcon).toBeVisible();

    // メニューを閉じる
    await closeMapMenu(page, browserName);
  });
});