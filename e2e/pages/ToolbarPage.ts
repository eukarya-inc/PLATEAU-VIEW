import { Page, expect } from '@playwright/test';
import { BasePage } from './BasePage';

export class ToolbarPage extends BasePage {
  // data-name属性のマッピング
  public dataNameMapping: { [key: string]: string } = {
    'Myデータ': 'my-data',
    'ヘルプ': 'help',
    'フィードバック': 'feedback',
    'UIを隠す': 'hide-ui',
    // '3D都市モデルダウンロード'はdata-name属性を持たない
  };

  // ツールバー関連のセレクター
  protected toolbarSelectors = {
    buttons: {
      menuButton: 'メインメニュー',
      streetView: '歩行者視点',
      drawing: '作図',
      graphics: 'グラフィック設定',
      dateTime: '日時',
      map: '地図',
      story: 'ストーリー',
      share: 'シェア',
      shortcut: 'ショートカット表示',
    },
    navigation: {
      currentLocation: '現在位置',
      autoRotate: '自動回転',
      zoomIn: '拡大',
      zoomOut: '縮小',
      compass: 'コンパス',
    },
    graphics: {
      quality: (value: string) => `[data-value="${value}"]`,
    },
    modals: {
      menu: '.MuiModal-root.MuiMenu-root',
      dialog: '.MuiModal-root.MuiDialog-root',
      myData: '.MuiModal-root:has-text("Myデータ")',
      help: '.MuiModal-root:has-text("ヘルプ")',
      feedback: '.MuiModal-root:has-text("フィードバック")',
    },
  };

  constructor(page: Page, browserName: string = 'chromium') {
    super(page, browserName);
  }

  /**
   * メニューボタンをクリック
   */
  async clickMenuButton() {
    await this.page.getByRole('button', { name: this.toolbarSelectors.buttons.menuButton }).click();
  }

  /**
   * メニューが開いているか確認
   */
  async isMenuOpen() {
    // MuiMenu-rootクラスを持つモーダルを探す
    const menu = this.page.locator('.MuiModal-root.MuiMenu-root');
    return await menu.isVisible();
  }

  /**
   * メニューアイテムをクリック
   */
  async clickMenuItem(itemName: string) {
    // 3D都市モデルダウンロードは特別処理（data-name属性なし）
    if (itemName === '3D都市モデルダウンロード') {
      await this.page.locator(`a[role="menuitem"]:has-text("${itemName}")`).click();
      return;
    }
    
    // 通常のメニューアイテム
    await this.page.getByRole('menuitem', { name: itemName }).click();
  }

  /**
   * Myデータモーダルが開いているか確認
   */
  async isMyDataModalOpen() {
    // Myデータモーダルを特定する
    // MuiMenu-rootクラスを持たない、Myデータテキストを含むモーダルを探す
    const modal = this.page.locator('.MuiModal-root:not(.MuiMenu-root)').filter({ hasText: 'Myデータ' });
    const count = await modal.count();
    return count > 0;
  }

  /**
   * モーダルを閉じる
   */
  async closeModal() {
    await this.page.getByRole('button', { name: 'close' }).click();
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
    const modal = this.page.locator(this.toolbarSelectors.modals.feedback);
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
    await this.page.getByRole('button', { name: this.toolbarSelectors.buttons.streetView }).click();
  }

  /**
   * 作図ボタンをクリック
   */
  async clickDrawingButton() {
    await this.page.getByRole('button', { name: this.toolbarSelectors.buttons.drawing }).click();
  }

  /**
   * グラフィック設定ボタンをクリック
   */
  async clickGraphicsButton() {
    await this.page.getByRole('button', { name: this.toolbarSelectors.buttons.graphics }).click();
  }

  /**
   * グラフィック設定を選択
   */
  async selectGraphicsQuality(quality: '低' | '中' | '高' | '最高') {
    await this.page.locator(this.toolbarSelectors.graphics.quality(quality)).click();
  }

  /**
   * 日時設定ボタンをクリック
   */
  async clickDateTimeButton() {
    await this.page.getByRole('button', { name: this.toolbarSelectors.buttons.dateTime }).click();
  }

  /**
   * 地図設定ボタンをクリック
   */
  async clickMapButton() {
    await this.page.getByRole('button', { name: this.toolbarSelectors.buttons.map }).click();
  }

  /**
   * ストーリーボタンをクリック
   */
  async clickStoryButton() {
    await this.page.getByRole('button', { name: this.toolbarSelectors.buttons.story }).click();
  }

  /**
   * シェアボタンをクリック
   */
  async clickShareButton() {
    await this.page.getByRole('button', { name: this.toolbarSelectors.buttons.share }).click();
  }

  /**
   * ショートカット表示ボタンをクリック
   */
  async clickShortcutButton() {
    await this.page.getByRole('button', { name: this.toolbarSelectors.buttons.shortcut }).click();
  }

  /**
   * ナビゲーションコントロール
   */
  async clickCurrentLocation() {
    await this.page.getByRole('button', { name: this.toolbarSelectors.navigation.currentLocation }).click();
  }

  async clickAutoRotate() {
    await this.page.getByRole('button', { name: this.toolbarSelectors.navigation.autoRotate }).click();
  }

  async clickZoomIn() {
    await this.page.getByRole('button', { name: this.toolbarSelectors.navigation.zoomIn }).click();
  }

  async clickZoomOut() {
    await this.page.getByRole('button', { name: this.toolbarSelectors.navigation.zoomOut }).click();
  }

  async clickCompass() {
    await this.page.getByRole('button', { name: this.toolbarSelectors.navigation.compass }).click();
  }

  /**
   * ツールバーボタンの要素を取得
   */
  getToolbarButton(buttonName: keyof typeof this.toolbarSelectors.buttons) {
    return this.page.getByRole('button', { name: this.toolbarSelectors.buttons[buttonName] });
  }

  /**
   * ナビゲーションボタンの要素を取得
   */
  getNavigationButton(buttonName: keyof typeof this.toolbarSelectors.navigation) {
    return this.page.getByRole('button', { name: this.toolbarSelectors.navigation[buttonName] });
  }
}