import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  workers: 1,
  timeout: 45000,
  use: {
    baseURL: 'http://127.0.0.1:43127',
    viewport: { width: 1360, height: 1000 },
    trace: 'retain-on-failure',
  },
  webServer: {
    command: 'python3 tests/e2e_server.py',
    url: 'http://127.0.0.1:43127/api/state',
    reuseExistingServer: false,
    timeout: 120000,
  },
});
