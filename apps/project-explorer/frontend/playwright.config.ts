import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests/browser', fullyParallel: true, timeout: 45_000,
  use: { baseURL: 'http://127.0.0.1:4177/doramcap/', viewport: { width: 1280, height: 900 }, screenshot: 'only-on-failure' },
  webServer: { command: 'node scripts/serve-test.mjs', url: 'http://127.0.0.1:4177/doramcap/', reuseExistingServer: false },
});
