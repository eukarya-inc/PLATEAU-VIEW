import { Page } from '@playwright/test';

/**
 * GPUなし環境での待機時間を調整するヘルパー関数
 * 環境変数 SLOW_LOAD が設定されている場合、待機時間を倍率で調整します
 *
 * 使用例:
 * - 通常環境: await waitFor(page, 3000) → 3秒待機
 * - SLOW_LOAD=3の場合: await waitFor(page, 3000) → 9秒待機
 */
export async function waitFor(page: Page, milliseconds: number): Promise<void> {
  // コンテキストから倍率を取得、なければ環境変数、それもなければ1
  const contextMultiplier = (page.context() as any).slowLoadMultiplier;
  const envMultiplier = process.env.SLOW_LOAD ? parseInt(process.env.SLOW_LOAD, 10) : 1;
  const multiplier = contextMultiplier || envMultiplier;

  await page.waitForTimeout(milliseconds * multiplier);
}

/**
 * Cesiumの3Dタイルや建築物が完全に読み込まれるまで待機する
 * スクリーンショットを定期的に取得し、画面が安定したら完了とみなす
 *
 * @param page - Playwrightのページオブジェクト
 * @param interval - スクリーンショット取得間隔（ミリ秒）
 * @param stableCount - 画面が安定したと判断する連続回数
 * @param maxWaitTime - 最大待機時間（ミリ秒）
 * @returns 最終的なスクリーンショット
 */
export async function waitForCesiumStable(
  page: Page,
  interval: number = process.env.CI ? 5000 : 2000,
  stableCount: number = 3,
  maxWaitTime: number = 120000 // 2分
): Promise<Buffer> {
  const startTime = Date.now();
  let previousScreenshot: Buffer | null = null;
  let noChangeCount = 0;

  console.log(`Waiting for Cesium to stabilize (interval: ${interval}ms, stable count: ${stableCount})`);

  while (true) {
    // タイムアウトチェック
    if (Date.now() - startTime > maxWaitTime) {
      console.warn(`Cesium stabilization timed out after ${maxWaitTime}ms`);
      break;
    }

    // スクリーンショットを取得（3Dビューエリアのみ）
    const canvas = page.locator('canvas').first();
    const screenshot = await canvas.screenshot();

    if (previousScreenshot) {
      // 前回のスクリーンショットと比較
      if (Buffer.compare(previousScreenshot, screenshot) === 0) {
        noChangeCount++;
        console.log(`Screen unchanged (${noChangeCount}/${stableCount})`);

        if (noChangeCount >= stableCount) {
          console.log('Cesium is stable!');
          return screenshot;
        }
      } else {
        // 変化があった場合はカウントをリセット
        noChangeCount = 0;
        console.log('Screen changed, resetting count');
      }
    }

    previousScreenshot = screenshot;
    await page.waitForTimeout(interval);
  }

  // タイムアウトした場合でも最後のスクリーンショットを返す
  return previousScreenshot || Buffer.from('');
}
