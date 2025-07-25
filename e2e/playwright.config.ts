import { defineConfig, devices } from '@playwright/test';

// GPUなし環境（GitHub Actions等）での待機時間倍率
const SLOW_LOAD_MULTIPLIER = process.env.SLOW_LOAD ? parseInt(process.env.SLOW_LOAD, 10) : 1;

// 動画記録設定：デフォルトは失敗時のみ、RECORD_VIDEO=trueで全て記録
const VIDEO_MODE = process.env.RECORD_VIDEO === 'true' ? 'on' : 'retain-on-failure';

export default defineConfig({
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  use: {
    baseURL: process.env.BASE_URL || 'https://plateauview.mlit.go.jp',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: VIDEO_MODE,
    // カスタムテストデータとして待機時間倍率を渡す
    slowLoadMultiplier: SLOW_LOAD_MULTIPLIER,
  },

  reporter: process.env.CI
    ? [['list'], ['html', { open: 'never' }], ['github']]
    : [['list'], ['html', { open: 'never' }]],

  timeout: 1000 * 60 * 10,

  projects: [
    {
      name: 'chrome',
      use: {
        ...devices['Desktop Chrome'],
        // Chromeは他のブラウザより遅いため、タイムアウトを延長
        actionTimeout: 120000,
        navigationTimeout: 180000,
      },
      timeout: 120000,
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
    },
  ],

});
