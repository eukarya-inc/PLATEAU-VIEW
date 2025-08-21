import { ToolbarPage } from './ToolbarPage';

export class MenuPage extends ToolbarPage {
  // 親クラスのdataNameMappingを使用
  // メニュー関連のセレクター
  protected menuSelectors = {
    menu: '[role="menu"]',
    myDataModal: {
      root: '.MuiModal-root:has-text("Myデータ")',
      title: 'h2:has-text("マイデータ")',
      localTab: 'button[role="tab"]:has-text("ローカルのデータから追加")',
      webTab: 'button[role="tab"]:has-text("Webから追加")',
      uploadArea: 'div:has-text("ここをクリックしてファイルを選択するか")',
    },
    helpModal: {
      root: '.MuiModal-root:last-child',
      title: 'h2:has-text("ヘルプ")',
      tabs: {
        ui: 'button[role="tab"]:has-text("UIを理解する")',
        map: 'button[role="tab"]:has-text("マップ操作")',
        layer: 'button[role="tab"]:has-text("レイヤー")',
        inspector: 'button[role="tab"]:has-text("インスペクター")',
      },
    },
    feedbackModal: {
      root: '.MuiModal-root:nth-of-type(2)',
      content: 'p',
    },
    menuItems: {
      myData: 'Myデータ',
      help: 'ヘルプ',
      feedback: 'フィードバック',
      download: '3D都市モデルダウンロード',
      hideUI: 'UIを隠す',
    },
  };


  /**
   * メニューを開く
   */
  async openMenu() {
    if (!(await this.isMenuOpen())) {
      await this.clickMenuButton();
      // メニューが完全に開くまで待機（Chromeは時間がかかる）
      await this.page.waitForTimeout(this.browserName === 'chrome' ? 3000 : 1000);
    }
  }

  /**
   * メニューを閉じる
   */
  async closeMenu() {
    if (await this.isMenuOpen()) {
      // ESCキーでメニューを閉じる
      await this.page.keyboard.press('Escape');
      await this.page.waitForTimeout(500);
      
      // ESCキーで閉じない場合はBackdropをクリック
      if (await this.isMenuOpen()) {
        const backdrop = this.page.locator('.MuiBackdrop-root');
        if (await backdrop.isVisible()) {
          await backdrop.click({ force: true });
          await this.page.waitForTimeout(500);
        }
      }
    }
  }

  /**
   * Myデータを開く
   */
  async openMyData() {
    await this.openMenu();
    await this.clickMenuItem(this.menuSelectors.menuItems.myData);
    await this.page.waitForTimeout(1000);
  }

  /**
   * ヘルプを開く（新しいタブ）
   */
  async openHelp() {
    await this.openMenu();
    
    // 新しいタブが開くのを待つためのPromise
    const pagePromise = this.page.context().waitForEvent('page');
    await this.clickMenuItem(this.menuSelectors.menuItems.help);
    
    const newPage = await pagePromise;
    await newPage.waitForLoadState();
    
    return newPage;
  }

  /**
   * フィードバックを開く
   */
  async openFeedback() {
    await this.openMenu();
    await this.clickMenuItem(this.menuSelectors.menuItems.feedback);
    await this.page.waitForTimeout(1000);
  }

  /**
   * 3D都市モデルダウンロードを開く（新しいタブ）
   */
  async openDownload() {
    await this.openMenu();
    
    // 新しいタブが開くのを待つためのPromise
    const pagePromise = this.page.context().waitForEvent('page');
    await this.clickMenuItem(this.menuSelectors.menuItems.download);
    
    const newPage = await pagePromise;
    await newPage.waitForLoadState();
    
    return newPage;
  }

  /**
   * Myデータのタブを切り替える
   */
  async switchMyDataTab(tabName: 'アップロードファイル' | 'ストレージ') {
    await this.page.getByRole('tab', { name: tabName }).click();
  }

  /**
   * フィードバックモーダルのテキストを取得
   */
  async getFeedbackModalText() {
    const modal = this.page.locator(this.menuSelectors.feedbackModal.root);
    return await modal.locator(this.menuSelectors.feedbackModal.content).textContent();
  }

  /**
   * メニューアイテムが表示されているか確認
   */
  async hasMenuItem(itemName: string) {
    // 3D都市モデルダウンロードはリンク要素だがrole="menuitem"
    if (itemName === '3D都市モデルダウンロード') {
      // aタグでrole="menuitem"を持つ要素を探す
      const link = this.page.locator(`a[role="menuitem"]:has-text("${itemName}")`);
      return await link.isVisible();
    }
    
    // 通常のメニューアイテム
    const dataName = this.dataNameMapping[itemName] || itemName;
    const menuItem = this.menuItem(dataName);
    return await menuItem.isVisible();
  }

  /**
   * 全メニューアイテムの検証
   */
  async verifyAllMenuItems() {
    const expectedItems = Object.values(this.menuSelectors.menuItems).filter(item => item !== 'UIを隠す');
    const results: { [key: string]: boolean } = {};
    
    for (const item of expectedItems) {
      results[item] = await this.hasMenuItem(item);
    }
    
    return results;
  }


  /**
   * Myデータモーダルの要素を取得
   */
  getMyDataModal() {
    return this.page.locator(this.menuSelectors.myDataModal.root);
  }

  /**
   * ヘルプモーダルの要素を取得
   */
  getHelpModal() {
    return this.page.locator(this.menuSelectors.helpModal.root);
  }

  /**
   * フィードバックモーダルの要素を取得
   */
  getFeedbackModal() {
    return this.page.locator(this.menuSelectors.feedbackModal.root);
  }
}