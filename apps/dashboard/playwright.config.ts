import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 60_000,
  retries: 0,
  workers: 1,
  use: {
    baseURL: 'http://127.0.0.1:5173',
    screenshot: 'only-on-failure',
  },
  webServer: [
    {
      command: 'cd ../.. && ./scripts/dashboard-dev.sh',
      url: 'http://127.0.0.1:7433/api/v1/health',
      reuseExistingServer: true,
      timeout: 240_000,
      stdout: 'ignore',
      stderr: 'ignore',
    },
  ],
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  ],
});
