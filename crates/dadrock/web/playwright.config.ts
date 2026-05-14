import { defineConfig } from '@playwright/test';

const devServerCommand =
  process.env.E2E_WEB_SERVER_COMMAND ??
  'VITE_DADR_MOCK=1 node ./node_modules/vite/bin/vite.js dev --host 127.0.0.1';

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: 'list',
  use: {
    baseURL: process.env.E2E_BASE_URL ?? 'http://127.0.0.1:5173',
    trace: 'retain-on-failure',
  },
  webServer: process.env.E2E_BASE_URL
    ? undefined
    : {
        command: devServerCommand,
        url: 'http://127.0.0.1:5173',
        timeout: 60_000,
        reuseExistingServer: !process.env.CI,
      },
});
