import { Page, Locator } from '@playwright/test';

export const DEFAULT_URL = 'https://plateauview.mlit.go.jp';

export class BasePage {
  page: Page;  // publicにしてテストから直接アクセス可能にする
  protected browserName: string;

  // 共通セレクター
  protected selectors = {
    menuButton: 'button[aria-label="メインメニュー"]',
    modalRoot: '.MuiModal-root',
    closeButton: 'button[aria-label="close"]',
    menuItem: (dataName: string) => `li[role="menuitem"][data-name="${dataName}"]`,
    button: (label: string) => `button[aria-label="${label}"]`,
    tab: (name: string) => `button[role="tab"]:has-text("${name}")`,
    searchInput: 'input[placeholder="データセット、建築物、住所を検索"]',
  };

  constructor(page: Page, browserName: string = 'chromium') {
    this.page = page;
    this.browserName = browserName;
  }

  /**
   * Chromium用のクリック処理
   * 並列実行時のChromiumの問題を回避するため
   */
  async clickElement(selector: string) {
    if (this.browserName === 'chromium' || this.browserName === 'firefox') {
      await this.page.evaluate((sel) => {
        const element = document.querySelector(sel) as HTMLElement;
        if (element) element.click();
      }, selector);
    } else {
      await this.page.locator(selector).click();
    }
  }

  /**
   * ロールベースの要素をクリック
   */
  async clickByRole(role: 'button' | 'menuitem' | 'link' | 'tab', name: string) {
    // ChromiumとFirefoxではJavaScriptクリックを使用
    if (this.browserName === 'chromium' || this.browserName === 'firefox') {
      let selector = '';
      if (role === 'button') {
        selector = this.selectors.button(name);
      } else if (role === 'menuitem') {
        // メニューアイテムの場合は、そのまま名前で検索
        // data-name属性の変換は各ページクラスで行う
        selector = this.selectors.menuItem(name);
      } else if (role === 'tab') {
        selector = this.selectors.tab(name);
      }
      if (selector) {
        await this.clickElement(selector);
      }
    } else {
      await this.page.getByRole(role, { name }).click();
    }
  }


  /**
   * ページへ遷移
   */
  async goto() {
    await this.page.goto(DEFAULT_URL);
  }

  /**
   * ページの初期化を待つ
   */
  async waitForPageReady() {
    await this.page.waitForLoadState('domcontentloaded');

    // メインメニューボタンが表示されるまで待機
    await this.page.waitForSelector(this.selectors.menuButton, {
      state: 'visible',
      timeout: 30000
    });
  }

  /**
   * 要素が表示されるまで待つ
   */
  async waitForVisible(locator: Locator, timeout: number = 5000) {
    await locator.waitFor({ state: 'visible', timeout });
  }

  /**
   * 要素が非表示になるまで待つ
   */
  async waitForHidden(locator: Locator, timeout: number = 5000) {
    await locator.waitFor({ state: 'hidden', timeout });
  }

  /**
   * 検索バーをクリック（ブラウザ対応）
   */
  async clickSearchInput() {
    await this.clickElement(this.selectors.searchInput);
  }

  /**
   * 検索バーをクリア（ブラウザ対応）
   */
  async clearSearchInput() {
    if (this.browserName === 'chromium' || this.browserName === 'firefox') {
      // JavaScriptで強制的にクリア
      await this.page.evaluate((selector) => {
        const element = document.querySelector(selector) as HTMLInputElement;
        if (element) {
          element.focus();
          element.select();
          document.execCommand('delete');
          // 念のため直接値をクリア
          element.value = '';
          element.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, this.selectors.searchInput);
    } else {
      const searchInput = this.page.locator(this.selectors.searchInput);
      await searchInput.clear();
    }
  }
}
