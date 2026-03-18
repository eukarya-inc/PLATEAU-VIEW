import '@playwright/test';

declare module '@playwright/test' {
  export interface PlaywrightTestOptions {
    slowLoadMultiplier?: number;
  }
}