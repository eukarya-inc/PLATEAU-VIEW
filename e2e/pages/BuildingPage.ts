import { BasePage } from './BasePage';

export class BuildingPage extends BasePage {

  /**
   * 選択モードに切り替える
   */
  async switchToSelectMode() {
    const selectButton = this.page.getByRole('button', { name: '選択' });
    await selectButton.click();
    await this.page.waitForTimeout(500);
  }

  /**
   * 移動モードに切り替える
   */
  async switchToMoveMode() {
    const moveButton = this.page.getByRole('button', { name: '移動' });
    await moveButton.click();
    await this.page.waitForTimeout(500);
  }

  /**
   * 3Dビューの中央をクリック
   */
  async clickCanvasCenter() {
    const canvas = this.page.locator('canvas').first();
    const box = await canvas.boundingBox();
    if (box) {
      await this.page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    }
  }

  /**
   * 属性パネルが表示されているか確認
   */
  async isAttributePanelVisible(): Promise<boolean> {
    const panel = this.getAttributePanel();
    return await panel.isVisible().catch(() => false);
  }

  /**
   * 属性パネルを取得
   */
  getAttributePanel() {
    // 属性パネルは画面下部に表示される。閉じるボタンがあるリストを探す
    return this.page.locator('ul').filter({ has: this.page.locator('button[aria-label="閉じる"]') }).first();
  }

  /**
   * 属性パネルを閉じる
   */
  async closeAttributePanel() {
    const panel = this.getAttributePanel();
    if (await panel.isVisible()) {
      // 閉じるボタンを探す
      const closeButton = panel.locator('button[aria-label*="閉じる"], button[aria-label*="close"], button:has(svg)').first();
      if (await closeButton.isVisible()) {
        await closeButton.click();
      } else {
        // ESCキーで閉じる
        await this.page.keyboard.press('Escape');
      }
      await this.page.waitForTimeout(500);
    }
  }

  /**
   * 特定の属性が表示されているか確認
   */
  async hasAttribute(attributeName: string): Promise<boolean> {
    const panel = this.getAttributePanel();
    const element = panel.getByText(attributeName);
    return await element.isVisible().catch(() => false);
  }

  /**
   * いずれかの建築物属性が表示されているか確認
   */
  async hasBuildingAttributes(): Promise<boolean> {
    const possibleAttributes = ['高さ', '階数', '用途', '住所', '建築年', '構造'];
    
    for (const attr of possibleAttributes) {
      if (await this.hasAttribute(attr)) {
        return true;
      }
    }
    return false;
  }
}