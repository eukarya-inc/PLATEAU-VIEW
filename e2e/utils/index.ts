import { Browser, BrowserContext, Page, TestInfo } from '@playwright/test';

/**
 * beforeAllでページを作成する際のヘルパー関数
 *
 * WORKAROUND: Playwrightの既知の問題により、beforeAllで作成したコンテキストには
 * 設定ファイルのvideo設定が自動的に適用されません。
 * この関数は設定を手動で適用するワークアラウンドです。
 *
 * 関連issues:
 * - https://github.com/microsoft/playwright/issues/11644 - When creating a new Page in beforeAll to share between tests
 * - https://github.com/microsoft/playwright/issues/14813 - Videos are not generated when reusing a single page between tests
 * - https://github.com/microsoft/playwright/issues/33720 - Video not attaching to HTML report when using browser from beforeAll
 */
export async function init(
  browser: Browser,
  testInfo: TestInfo
): Promise<{ context: BrowserContext; page: Page }> {
  const videoConfig = testInfo.project.use?.video;

  // テストファイル名を取得（拡張子なし）
  const testFileName = testInfo.file.split('/').pop()?.replace('.spec.ts', '') || 'test';
  const projectName = testInfo.project.name;
  // describeのタイトルを取得（@タグとその後のスペースを除去）
  // titlePath[0]はファイルパス、titlePath[1]がdescribeのタイトル
  const suiteTitle = testInfo.titlePath[1]?.replace(/@\w+\s*/g, '').trim() || 'test';

  // Playwrightのデフォルトディレクトリ構造を再現
  // 例: test-results/basic-スモークテスト-webkit/
  const context = await browser.newContext({
    recordVideo: (videoConfig && (videoConfig === 'on' || typeof videoConfig === 'object' && videoConfig.mode === 'on')) ? {
      dir: `./test-results/${testFileName}-${suiteTitle}-${projectName}`
    } : undefined
  });

  const page = await context.newPage();
  return { context, page };
}
