import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  use: {
    baseURL: 'https://plateauview.mlit.go.jp',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'on',
    actionTimeout: 30000,  // 10秒から30秒に増やす
    navigationTimeout: 60000,  // 30秒から60秒に増やす
  },

  reporter: [
    ['list'],
    ['html', { open: 'never' }]
  ],

  timeout: 60000,

  projects: [
    {
      name: 'chrome',
      use: { 
        ...devices['Desktop Chrome'],
        // Chromeは他のブラウザより遅いため、タイムアウトを延長
        actionTimeout: 60000,
        navigationTimeout: 90000,
        // ヘッドレスモードの最適化
        launchOptions: {
          args: ['--disable-blink-features=AutomationControlled']
        }
      },
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
