import { expect } from '@playwright/test';
import { BasePage } from './BasePage';

export class AttributesPage extends BasePage {
  // 属性ページ固有のセレクター定義
  private attributeSelectors = {
    selectButton: 'button[aria-label="選択"]',
    moveButton: 'button[aria-label="移動"]',
    canvas: 'canvas',
    attributePanel: 'ul',
    closeButton: 'button[aria-label="閉じる"]',
  };

  /**
   * 選択モードに切り替える
   */
  async switchToSelectMode() {
    const selectButton = this.page.getByRole('button', { name: '選択' });
    await selectButton.click();
    await expect(selectButton).toHaveAttribute('aria-pressed', 'true');
  }

  /**
   * 移動モードに切り替える
   */
  async switchToMoveMode() {
    const moveButton = this.page.getByRole('button', { name: '移動' });
    await moveButton.click();
    await expect(moveButton).toHaveAttribute('aria-pressed', 'true');
  }

  /**
   * 3Dビューの建築物をクリック
   * @param position - クリック位置の調整（0.5 = 中央、0.6 = 少し下）
   */
  async clickBuilding(position: number = 0.6) {
    const canvas = this.page.locator(this.attributeSelectors.canvas).first();
    const box = await canvas.boundingBox();
    if (!box) {
      throw new Error('Canvas bounding box is not available');
    }

    await this.page.mouse.click(box.x + box.width / 2, box.y + box.height * position);
  }

  /**
   * 属性パネルを取得
   */
  getAttributePanel() {
    return this.page.locator(this.attributeSelectors.attributePanel).filter({ 
      has: this.page.locator(this.attributeSelectors.closeButton) 
    });
  }

  /**
   * 属性パネルが表示されているか確認
   */
  async isAttributePanelVisible(): Promise<boolean> {
    return await this.getAttributePanel().isVisible();
  }

  /**
   * 属性パネルに特定の属性が含まれているか確認
   */
  async hasAttributes(attributes: string[]): Promise<boolean> {
    const panel = this.getAttributePanel();
    const panelText = await panel.textContent();
    
    if (!panelText) return false;
    
    return attributes.some(attr => panelText.includes(attr));
  }

  /**
   * 属性パネルに建築物の属性が表示されているか確認
   */
  async hasBuildingAttributes(): Promise<boolean> {
    const buildingAttributes = ['高さ', '階数', '用途', '住所'];
    return await this.hasAttributes(buildingAttributes);
  }

  /**
   * 属性パネルを閉じる
   */
  async closeAttributePanel() {
    const panel = this.getAttributePanel();
    const closeButton = panel.locator(this.attributeSelectors.closeButton);
    await closeButton.click();
  }

  /**
   * 属性パネルが表示されるまで待つ
   */
  async waitForAttributePanel() {
    await expect(this.getAttributePanel()).toBeVisible();
  }

  /**
   * 属性パネルが非表示になるまで待つ
   */
  async waitForAttributePanelToHide() {
    await expect(this.getAttributePanel()).not.toBeVisible();
  }
}