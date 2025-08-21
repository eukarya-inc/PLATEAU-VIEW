import { Locator, Page, Browser } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 3Dビューキャンバスの操作を担当するページオブジェクト
 */
export class CanvasPage extends BasePage {
  readonly canvas: Locator;

  constructor(page: Page, browser: Browser) {
    super(page, browser);
    
    // キャンバスのLocator初期化
    this.canvas = page.locator('canvas').first();
  }

  /**
   * キャンバスの特定位置をクリック
   * @param x - X座標の比率（0.0-1.0）
   * @param y - Y座標の比率（0.0-1.0）
   */
  async clickAt(x: number = 0.5, y: number = 0.5) {
    const box = await this.canvas.boundingBox();
    if (!box) {
      throw new Error('Canvas bounding box is not available');
    }

    await this.page.mouse.click(box.x + box.width * x, box.y + box.height * y);
  }

  /**
   * キャンバスの中央をクリック
   */
  async clickCenter() {
    await this.clickAt(0.5, 0.5);
  }

  /**
   * キャンバス上でドラッグ操作
   * @param startX - 開始X座標の比率
   * @param startY - 開始Y座標の比率
   * @param endX - 終了X座標の比率
   * @param endY - 終了Y座標の比率
   */
  async drag(startX: number, startY: number, endX: number, endY: number) {
    const box = await this.canvas.boundingBox();
    if (!box) {
      throw new Error('Canvas bounding box is not available');
    }

    const fromX = box.x + box.width * startX;
    const fromY = box.y + box.height * startY;
    const toX = box.x + box.width * endX;
    const toY = box.y + box.height * endY;

    await this.page.mouse.move(fromX, fromY);
    await this.page.mouse.down();
    await this.page.mouse.move(toX, toY);
    await this.page.mouse.up();
  }

  /**
   * マウスホイールでズーム
   * @param deltaY - スクロール量（正の値でズームアウト、負の値でズームイン）
   */
  async zoom(deltaY: number) {
    const box = await this.canvas.boundingBox();
    if (!box) {
      throw new Error('Canvas bounding box is not available');
    }

    await this.page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await this.page.mouse.wheel(0, deltaY);
  }

  /**
   * キャンバスが表示されているか確認
   */
  async isVisible(): Promise<boolean> {
    return await this.canvas.isVisible();
  }
}