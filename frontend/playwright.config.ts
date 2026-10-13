// End-to-end browser tests against the running docker stack
// (`docker compose up --build` serves the whole app on :5317). Set
// E2E_BASE_URL to point somewhere else, e.g. a vite dev server proxying a
// local API. The suites seed and inspect the database through the
// loganalyzer-postgres container (e2e/helpers.ts), so they run one at a
// time.
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: 'e2e',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [['list']],
  use: {
    baseURL: process.env.E2E_BASE_URL ?? 'http://localhost:5317',
    // The log times are UTC; the page shows them in the visitor's zone.
    locale: 'en-US',
    timezoneId: 'UTC',
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  projects: [{ name: 'chromium', use: { browserName: 'chromium' } }],
});
