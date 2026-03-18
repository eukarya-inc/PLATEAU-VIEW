import { test } from '@playwright/test';
import { SearchPage } from '../../pages';
import { init } from '../../utils';

test.describe('検索機能 @smoke', () => {
  let searchPage: SearchPage;

  test.beforeAll(async ({ browser }, testInfo) => {
    const { page } = await init(browser, testInfo);
    searchPage = new SearchPage(page, browser);
    
    // ページに遷移して初期化を待つ
    await searchPage.goto();
    await searchPage.waitForPageReady();
  });

  test('地域名で検索するとデータセットが表示される', async () => {
    // 「千代田区」で検索
    await searchPage.searchFor('千代田区');

    // 「交通（道路）モデル（千代田区）」が表示され、詳細情報が正しいことを確認
    await searchPage.verifySearchResultDetails(
      '交通（道路）モデル（千代田区）',
      '交通（道路）モデル（千代田区）',
      '東京都 千代田区'
    );
  });
});