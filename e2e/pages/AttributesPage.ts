import { expect, Locator, Page, Browser } from '@playwright/test';
import { BasePage } from './BasePage';

/**
 * 属性パネルの操作を担当するページオブジェクト
 */
export class AttributesPage extends BasePage {
  private readonly attributePanelCloseButton: Locator;

  constructor(page: Page, browser: Browser) {
    super(page, browser);
    
    // Locatorの初期化
    this.attributePanelCloseButton = page.getByRole('button', { name: '閉じる' });
  }

  /**
   * 属性パネルを取得
   */
  getAttributePanel(): Locator {
    return this.page.locator('ul').filter({ 
      has: this.attributePanelCloseButton
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
    await panel.getByRole('button', { name: '閉じる' }).click();
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