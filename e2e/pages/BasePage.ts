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
   * 検索バーをクリック
   */
  async clickSearchInput() {
    await this.page.locator(this.selectors.searchInput).click();
  }

  /**
   * 検索バーをクリア
   */
  async clearSearchInput() {
    const searchInput = this.page.locator(this.selectors.searchInput);
    await searchInput.clear();
  }
}
