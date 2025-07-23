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
    await this.clickByRole('button', this.toolbarSelectors.buttons.menuButton);
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
      // aタグでrole="menuitem"
      if (this.browserName === 'chromium' || this.browserName === 'firefox') {
        await this.clickElement(`a[role="menuitem"]:has-text("${itemName}")`);
      } else {
        await this.page.locator(`a[role="menuitem"]:has-text("${itemName}")`).click();
      }
      return;
    }
    
    // data-name属性を使ってクリック
    const dataName = this.dataNameMapping[itemName] || itemName;
    if (this.browserName === 'chromium' || this.browserName === 'firefox') {
      await this.clickElement(this.selectors.menuItem(dataName));
    } else {
      await this.page.getByRole('menuitem', { name: itemName }).click();
    }
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
    await this.clickByRole('button', 'close');
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
    await this.clickByRole('button', this.toolbarSelectors.buttons.streetView);
  }

  /**
   * 作図ボタンをクリック
   */
  async clickDrawingButton() {
    await this.clickByRole('button', this.toolbarSelectors.buttons.drawing);
  }

  /**
   * グラフィック設定ボタンをクリック
   */
  async clickGraphicsButton() {
    await this.clickByRole('button', this.toolbarSelectors.buttons.graphics);
  }

  /**
   * グラフィック設定を選択
   */
  async selectGraphicsQuality(quality: '低' | '中' | '高' | '最高') {
    await this.clickElement(this.toolbarSelectors.graphics.quality(quality));
  }

  /**
   * 日時設定ボタンをクリック
   */
  async clickDateTimeButton() {
    await this.clickByRole('button', this.toolbarSelectors.buttons.dateTime);
  }

  /**
   * 地図設定ボタンをクリック
   */
  async clickMapButton() {
    await this.clickByRole('button', this.toolbarSelectors.buttons.map);
  }

  /**
   * ストーリーボタンをクリック
   */
  async clickStoryButton() {
    await this.clickByRole('button', this.toolbarSelectors.buttons.story);
  }

  /**
   * シェアボタンをクリック
   */
  async clickShareButton() {
    await this.clickByRole('button', this.toolbarSelectors.buttons.share);
  }

  /**
   * ショートカット表示ボタンをクリック
   */
  async clickShortcutButton() {
    await this.clickByRole('button', this.toolbarSelectors.buttons.shortcut);
  }

  /**
   * ナビゲーションコントロール
   */
  async clickCurrentLocation() {
    await this.clickByRole('button', this.toolbarSelectors.navigation.currentLocation);
  }

  async clickAutoRotate() {
    await this.clickByRole('button', this.toolbarSelectors.navigation.autoRotate);
  }

  async clickZoomIn() {
    await this.clickByRole('button', this.toolbarSelectors.navigation.zoomIn);
  }

  async clickZoomOut() {
    await this.clickByRole('button', this.toolbarSelectors.navigation.zoomOut);
  }

  async clickCompass() {
    await this.clickByRole('button', this.toolbarSelectors.navigation.compass);
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