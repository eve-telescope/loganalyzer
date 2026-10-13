// The upload page: the form, its client-side size check, the server's
// validation messages, the progress bar and the way to the analysis.
import { expect, test } from '@playwright/test';

import { cacheNames, fixture, psql, resetDatabase, watchErrors } from './helpers.ts';

test.beforeEach(() => resetDatabase());

test('uploads a log and opens its analysis', async ({ page }) => {
  const errors = watchErrors(page);
  cacheNames('testlog');
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'COMBAT LOG ANALYZER' })).toBeVisible();
  const submit = page.getByRole('button', { name: 'ANALYZE COMBAT LOG' });
  await expect(submit).toBeDisabled();
  await page.locator('input[type=file]').setInputFiles({
    name: 'my-fight.txt',
    mimeType: 'text/plain',
    buffer: fixture('testlog'),
  });
  await expect(page.getByText('my-fight.txt')).toBeVisible();
  await submit.click();
  await page.waitForURL(/\/logs\/[0-9a-f-]{36}$/);
  await expect(page.getByRole('heading', { name: 'Nicolas Kion' })).toBeVisible();
  expect(psql('select original_filename from combat_logs')).toBe('my-fight.txt');
  expect(Number(psql('select count(*) from combat_events'))).toBe(120);
  expect(errors).toEqual([]);
});

test('shows the upload progress', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/');
  // A slow network makes the progress visible.
  const client = await page.context().newCDPSession(page);
  await client.send('Network.emulateNetworkConditions', {
    offline: false,
    latency: 50,
    downloadThroughput: 4 * 1024 * 1024,
    uploadThroughput: 1024 * 1024,
  });
  await page.locator('input[type=file]').setInputFiles({
    name: 'big.txt',
    mimeType: 'text/plain',
    buffer: Buffer.concat(Array.from({ length: 16 }, () => fixture('testlog3'))),
  });
  await page.getByRole('button', { name: 'ANALYZE COMBAT LOG' }).click();
  await expect(page.getByText(/UPLOADING\.\.\. \d+%/)).toBeVisible();
  await expect(page.getByRole('button', { name: 'ANALYZING...' })).toBeDisabled();
  await page.waitForURL(/\/logs\//, { timeout: 60_000 });
});

test('refuses a file over the limit before sending it', async ({ page }) => {
  await page.goto('/');
  await page.locator('input[type=file]').setInputFiles({
    name: 'huge.txt',
    mimeType: 'text/plain',
    buffer: Buffer.alloc(25 * 1024 * 1024 + 1, 'a'),
  });
  await expect(page.getByText('File is too large. Maximum size is 25MB.')).toBeVisible();
  await expect(page.getByRole('button', { name: 'ANALYZE COMBAT LOG' })).toBeDisabled();
  expect(psql('select count(*) from combat_logs')).toBe('0');
});

test('shows the server validation message', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await page.locator('input[type=file]').setInputFiles({
    name: 'fight.pdf',
    mimeType: 'application/pdf',
    buffer: fixture('testlog'),
  });
  await page.getByRole('button', { name: 'ANALYZE COMBAT LOG' }).click();
  await expect(
    page.getByText('The log file field must have one of the following extensions: txt, log.'),
  ).toBeVisible();
  await expect(page).toHaveURL(/\/$/);
  expect(psql('select count(*) from combat_logs')).toBe('0');
  expect(errors).toEqual([]);
});

test('tells the limit of the upload', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByText('.txt or .log files up to 25MB')).toBeVisible();
});
