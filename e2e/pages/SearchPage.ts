import { Page, Locator, Browser, expect } from '@playwright/test';
import { BasePage } from './BasePage';

export class SearchPage extends BasePage {
  // 検索関連のLocator
  readonly searchBox: Locator;
  readonly searchListbox: Locator;
  readonly searchOptions: Locator;
  readonly clearButton: Locator;

  constructor(page: Page, browser: Browser) {
    super(page, browser);

    // 検索ボックス関連のLocator
    this.searchBox = page.getByRole('combobox', { name: 'データセット、建築物、住所を検索' });
    this.searchListbox = page.getByRole('listbox');
    this.searchOptions = page.getByRole('option');
    this.clearButton = page.getByRole('button', { name: 'Clear' });
  }

  /**
   * 検索ボックスを開く
   */
  async openSearchBox() {
    await this.searchBox.click();
    await expect(this.searchBox).toBeFocused();
  }

  /**
   * 検索キーワードを入力
   */
  async searchFor(keyword: string) {
    await this.openSearchBox();
    await this.searchBox.fill(keyword);
    
    // 検索APIの応答を待つ
    await this.page.waitForTimeout(2000);
    
    // 検索結果が表示されるまで待機
    await this.searchOptions.first().waitFor({
      state: 'visible',
      timeout: 10000
    });
  }

  /**
   * 検索ボックスをクリア
   */
  async clearSearch() {
    if (await this.clearButton.isVisible()) {
      await this.clearButton.click();
    } else {
      await this.searchBox.clear();
    }
  }

  /**
   * 検索結果のオプションを取得
   */
  getSearchOption(text: string): Locator {
    return this.searchOptions.filter({ hasText: text });
  }

  /**
   * 最初の検索結果を取得
   */
  getFirstSearchOption(): Locator {
    return this.searchOptions.first();
  }

  /**
   * 特定のデータセットの検索結果を取得
   */
  getDatasetOption(datasetName: string): Locator {
    return this.getSearchOption(datasetName).first();
  }

  /**
   * 検索結果が表示されているか確認
   */
  async hasSearchResults(): Promise<boolean> {
    return await this.searchOptions.count() > 0;
  }

  /**
   * 特定の検索結果が表示されているか確認
   */
  async hasSearchResult(text: string): Promise<boolean> {
    const option = this.getSearchOption(text);
    return await option.count() > 0;
  }

  /**
   * 検索結果をクリック
   */
  async clickSearchResult(text: string) {
    const option = this.getSearchOption(text).first();
    await option.click();
  }

  /**
   * 検索結果の詳細テキストを検証
   */
  async verifySearchResultDetails(optionText: string, expectedTitle: string, expectedLocation: string) {
    const option = this.getDatasetOption(optionText);
    
    // オプションが表示されていることを確認
    await expect(option).toBeVisible();
    
    // タイトルテキストを確認
    const titleText = option.locator('p').first();
    await expect(titleText).toHaveText(expectedTitle);
    
    // ロケーションテキストを確認
    const locationText = option.locator('p').nth(1);
    await expect(locationText).toHaveText(expectedLocation);
  }

  /**
   * 検索ボックスが開いているか確認
   */
  async isSearchBoxOpen(): Promise<boolean> {
    const expandedState = await this.searchBox.getAttribute('aria-expanded');
    return expandedState === 'true';
  }

  /**
   * 検索ボックスを閉じる
   */
  async closeSearchBox() {
    if (await this.isSearchBoxOpen()) {
      await this.page.keyboard.press('Escape');
      await this.page.waitForTimeout(500);
    }
  }
}