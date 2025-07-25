import { Page, Locator, Browser } from '@playwright/test';

export const DEFAULT_URL = 'https://plateauview.mlit.go.jp';

export class BasePage {
  readonly page: Page;
  protected readonly browser: Browser;
  protected readonly browserName: string;

  // 共通Locator
  readonly menuButton: Locator;
  readonly modalRoot: Locator;
  readonly closeButton: Locator;
  readonly searchInput: Locator;

  constructor(page: Page, browser: Browser) {
    this.page = page;
    this.browser = browser;
    this.browserName = browser.browserType().name();

    // Locatorの初期化
    this.menuButton = page.getByRole('button', { name: 'メインメニュー' });
    this.modalRoot = page.locator('.MuiModal-root');
    this.closeButton = page.getByRole('button', { name: 'close' });
    this.searchInput = page.getByPlaceholder('データセット、建築物、住所を検索');
  }

  // 動的Locatorを返すメソッド
  menuItem(dataName: string): Locator {
    return this.page.locator(`li[role="menuitem"][data-name="${dataName}"]`);
  }

  button(label: string): Locator {
    return this.page.getByRole('button', { name: label });
  }

  tab(name: string): Locator {
    return this.page.getByRole('tab', { name });
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
    await this.menuButton.waitFor({
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
    await this.searchInput.click();
  }

  /**
   * 検索バーをクリア
   */
  async clearSearchInput() {
    await this.searchInput.clear();
  }
}
