import { Locator, expect, Page, Browser } from '@playwright/test';
import { BasePage } from './BasePage';

export class ToolbarPage extends BasePage {
  // ツールモード関連のLocator
  readonly selectButton: Locator;
  readonly moveButton: Locator;
  // data-name属性のマッピング
  public dataNameMapping: { [key: string]: string } = {
    'Myデータ': 'my-data',
    'ヘルプ': 'help',
    'フィードバック': 'feedback',
    'UIを隠す': 'hide-ui',
    // '3D都市モデルダウンロード'はdata-name属性を持たない
  };

  // ツールバーボタン名の定数
  protected readonly buttonNames = {
    menuButton: 'メインメニュー',
    streetView: '歩行者視点',
    drawing: '作図',
    graphics: 'グラフィック設定',
    dateTime: '日時',
    map: '地図',
    story: 'ストーリー',
    share: 'シェア',
    shortcut: 'ショートカット表示',
  };

  // ナビゲーションボタン名の定数
  protected readonly navigationNames = {
    currentLocation: '現在位置',
    autoRotate: '自動回転',
    zoomIn: '拡大',
    zoomOut: '縮小',
    compass: 'コンパス',
  };

  constructor(page: Page, browser: Browser) {
    super(page, browser);
    
    // ツールモードボタンの初期化
    this.selectButton = page.getByRole('button', { name: '選択' });
    this.moveButton = page.getByRole('button', { name: '移動' });
  }

  // 動的Locatorを返すメソッド
  getMenu(): Locator {
    return this.page.locator('.MuiModal-root.MuiMenu-root');
  }

  getDialog(): Locator {
    return this.page.locator('.MuiModal-root.MuiDialog-root');
  }

  getMyDataModal(): Locator {
    return this.page.locator('.MuiModal-root').filter({ hasText: 'Myデータ' });
  }

  getHelpModal(): Locator {
    return this.page.locator('.MuiModal-root').filter({ hasText: 'ヘルプ' });
  }

  getFeedbackModal(): Locator {
    return this.page.locator('.MuiModal-root').filter({ hasText: 'フィードバック' });
  }


  /**
   * 選択モードに切り替える
   */
  async switchToSelectMode() {
    await this.selectButton.click();
    await expect(this.selectButton).toHaveAttribute('aria-pressed', 'true');
  }

  /**
   * 移動モードに切り替える
   */
  async switchToMoveMode() {
    await this.moveButton.click();
    await expect(this.moveButton).toHaveAttribute('aria-pressed', 'true');
  }

  /**
   * メニューボタンをクリック
   */
  async clickMenuButton() {
    await this.menuButton.click();
  }

  /**
   * メニューが開いているか確認
   */
  async isMenuOpen() {
    return await this.getMenu().isVisible();
  }

  /**
   * メニューアイテムをクリック
   */
  async clickMenuItem(itemName: string) {
    // 3D都市モデルダウンロードは特別処理（data-name属性なし）
    if (itemName === '3D都市モデルダウンロード') {
      await this.page.getByRole('menuitem', { name: itemName }).click();
      return;
    }
    
    // 通常のメニューアイテム
    await this.page.getByRole('menuitem', { name: itemName }).click();
  }

  /**
   * Myデータモーダルが開いているか確認
   */
  async isMyDataModalOpen() {
    const count = await this.getMyDataModal().count();
    return count > 0;
  }

  /**
   * モーダルを閉じる
   */
  async closeModal() {
    await this.closeButton.click();
  }

  /**
   * ヘルプタブに切り替わったか確認
   */
  async isHelpTabOpen() {
    // ヘルプリンクがクリックされると新しいタブが開く
    const pages = this.page.context().pages();
    return pages.length > 1;
  }

  /**
   * フィードバックモーダルが開いているか確認
   */
  async isFeedbackModalOpen() {
    const modal = this.getFeedbackModal();
    return await modal.isVisible();
  }

  /**
   * 任意のモーダルが開いているか確認（メニュー以外）
   */
  async isModalOpen() {
    const modals = this.page.locator('.MuiModal-root:not(.MuiMenu-root)');
    const count = await modals.count();
    return count > 0;
  }

  /**
   * 歩行者視点ボタンをクリック
   */
  async clickStreetViewButton() {
    await this.button(this.buttonNames.streetView).click();
  }

  /**
   * 作図ボタンをクリック
   */
  async clickDrawingButton() {
    await this.button(this.buttonNames.drawing).click();
  }

  /**
   * グラフィック設定ボタンをクリック
   */
  async clickGraphicsButton() {
    await this.button(this.buttonNames.graphics).click();
  }

  /**
   * グラフィック設定を選択
   */
  async selectGraphicsQuality(quality: '低' | '中' | '高' | '最高') {
    await this.page.locator(`[data-value="${quality}"]`).click();
  }

  /**
   * 日時設定ボタンをクリック
   */
  async clickDateTimeButton() {
    await this.button(this.buttonNames.dateTime).click();
  }

  /**
   * 地図設定ボタンをクリック
   */
  async clickMapButton() {
    await this.button(this.buttonNames.map).click();
  }

  /**
   * ストーリーボタンをクリック
   */
  async clickStoryButton() {
    await this.button(this.buttonNames.story).click();
  }

  /**
   * シェアボタンをクリック
   */
  async clickShareButton() {
    await this.button(this.buttonNames.share).click();
  }

  /**
   * ショートカット表示ボタンをクリック
   */
  async clickShortcutButton() {
    await this.button(this.buttonNames.shortcut).click();
  }

  /**
   * ナビゲーションコントロール
   */
  async clickCurrentLocation() {
    await this.button(this.navigationNames.currentLocation).click();
  }

  async clickAutoRotate() {
    await this.button(this.navigationNames.autoRotate).click();
  }

  async clickZoomIn() {
    await this.button(this.navigationNames.zoomIn).click();
  }

  async clickZoomOut() {
    await this.button(this.navigationNames.zoomOut).click();
  }

  async clickCompass() {
    await this.button(this.navigationNames.compass).click();
  }

  /**
   * ツールバーボタンの要素を取得
   */
  getToolbarButton(buttonName: keyof typeof this.buttonNames) {
    return this.button(this.buttonNames[buttonName]);
  }

  /**
   * ナビゲーションボタンの要素を取得
   */
  getNavigationButton(buttonName: keyof typeof this.navigationNames) {
    return this.button(this.navigationNames[buttonName]);
  }
}