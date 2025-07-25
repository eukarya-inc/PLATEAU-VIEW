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
