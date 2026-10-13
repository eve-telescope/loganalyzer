// The analysis page: a shared link with all four URL parameters, the
// drag selection on the timeline, the series toggles, the pilot dialog,
// the tabs, the raw log download, the regeneration and a log without a
// raw file.
import { expect, test, type Page } from '@playwright/test';

import { TESTLOG } from './visual-pages.ts';
import {
  blockEveImages,
  deleteRawLog,
  fixture,
  psql,
  resetDatabase,
  uploadLog,
  watchErrors,
} from './helpers.ts';

test.beforeEach(() => resetDatabase());

function query(page: Page): URLSearchParams {
  return new URL(page.url()).searchParams;
}

test('opens a shared link with a range, hidden series and a pilot', async ({
  page,
  request,
  baseURL,
}) => {
  const errors = watchErrors(page);
  await blockEveImages(page);
  const uuid = await uploadLog(request, baseURL!, TESTLOG);
  await page.goto(
    `/logs/${uuid}?from=2026-02-12T10:20:00&to=2026-02-12T10:25:00&hide=logiDealt,neutOut&pilot=Aria%20Vex`,
  );
  await expect(page.getByText('Range: 10:20:00 - 10:25:00')).toBeVisible();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText('Engagement — Aria Vex')).toBeVisible();
  await expect(dialog.getByText('10:20:04')).toBeVisible();
  // The link stays as it was until something changes.
  expect(query(page).get('hide')).toBe('logiDealt,neutOut');

  await dialog.getByRole('button', { name: 'Close ✕' }).click();
  await expect(dialog).toBeHidden();
  await expect.poll(() => query(page).get('pilot')).toBeNull();
  expect(query(page).get('from')).toBe('2026-02-12T10:20:00');
  expect(query(page).get('hide')).toBe('logiDealt,neutOut');

  await page.getByRole('button', { name: 'Clear selection' }).click();
  await expect(page.getByText(/Range: /)).toBeHidden();
  await expect.poll(() => query(page).get('from')).toBeNull();
  expect(errors).toEqual([]);
});

test('selects a time range by dragging over the timeline', async ({ page, request, baseURL }) => {
  await blockEveImages(page);
  const uuid = await uploadLog(request, baseURL!, TESTLOG);
  await page.goto(`/logs/${uuid}`);
  const chart = page.getByTestId('dps-chart');
  await chart.scrollIntoViewIfNeeded();
  const box = (await chart.boundingBox())!;
  await page.mouse.move(box.x + box.width * 0.3, box.y + box.height * 0.6);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width * 0.6, box.y + box.height * 0.6, { steps: 8 });
  await page.mouse.up();
  await expect(page.getByText(/Range: \d\d:\d\d:\d\d - \d\d:\d\d:\d\d/)).toBeVisible();
  await expect.poll(() => query(page).get('from')).toMatch(/^2026-02-12T\d\d:\d\d:\d\d$/);
  const from = query(page).get('from')!;
  const to = query(page).get('to')!;
  expect(from < to).toBe(true);
  await expect(page.locator('header')).toContainText('selected');

  // A click without a drag clears it.
  await page.mouse.click(box.x + box.width * 0.5, box.y + box.height * 0.6);
  await expect.poll(() => query(page).get('from')).toBeNull();
});

test('hides and shows a series from the legend', async ({ page, request, baseURL }) => {
  await blockEveImages(page);
  const uuid = await uploadLog(request, baseURL!, TESTLOG);
  await page.goto(`/logs/${uuid}`);
  const canvas = page.getByTestId('dps-chart').locator('canvas');
  await canvas.scrollIntoViewIfNeeded();
  const box = (await canvas.boundingBox())!;
  // The legend is the first row of the chart; find an entry by clicking
  // along it.
  let hidden: string | null = null;
  for (let x = box.x + 20; x < box.x + box.width && !hidden; x += 12) {
    await page.mouse.click(x, box.y + 20);
    await page.waitForTimeout(150);
    hidden = query(page).get('hide');
  }
  expect(['dpsDealt', 'dpsReceived', 'logiReceived', 'logiDealt', 'neutIn', 'neutOut']).toContain(
    hidden,
  );
  await page.reload();
  await page.waitForLoadState('networkidle');
  expect(page.url()).toContain(`hide=${hidden}`);
});

test('opens a pilot from the tables and switches tabs', async ({ page, request, baseURL }) => {
  await blockEveImages(page);
  const uuid = await uploadLog(request, baseURL!, TESTLOG);
  await page.goto(`/logs/${uuid}`);
  await page.getByRole('row', { name: /Korr Malan/ }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog.getByText('Engagement — Korr Malan')).toBeVisible();
  await expect.poll(() => query(page).get('pilot')).toBe('Korr Malan');
  // Filter the engagement log.
  await dialog.getByRole('button', { name: 'incoming', exact: true }).click();
  await expect(dialog.getByText('No events match the filters')).toBeVisible();
  await dialog.getByRole('button', { name: 'both', exact: true }).click();
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();
  await expect.poll(() => query(page).get('pilot')).toBeNull();

  await page.getByRole('button', { name: 'Incoming', exact: true }).click();
  await expect(page.getByRole('row', { name: /Tovak Rin/ })).toBeVisible();
  await page.getByRole('button', { name: 'Logistics', exact: true }).click();
  await expect(page.getByRole('cell', { name: 'Received' }).first()).toBeVisible();
  await expect(page.getByRole('columnheader', { name: 'HP Repaired' })).toBeVisible();
});

test('downloads the raw log as it was uploaded', async ({ page, request, baseURL }) => {
  const uuid = await uploadLog(request, baseURL!, TESTLOG);
  await page.goto(`/logs/${uuid}`);
  const download = page.waitForEvent('download');
  await page.getByRole('link', { name: 'Raw Log' }).click();
  const file = await download;
  expect(file.suggestedFilename()).toBe('my-fight.txt');
  const path = await file.path();
  const { readFileSync } = await import('node:fs');
  expect(readFileSync(path).equals(fixture('testlog'))).toBe(true);
});

test('regenerates the analysis from the raw log', async ({ page, request, baseURL }) => {
  const errors = watchErrors(page);
  const uuid = await uploadLog(request, baseURL!, TESTLOG);
  const before = psql('select min(id) from combat_events');
  psql(`update combat_logs set listener = 'Stale Listener' where uuid = '${uuid}'`);
  await page.goto(`/logs/${uuid}?pilot=Aria%20Vex`);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('heading', { name: 'Stale Listener' })).toBeVisible();
  await page.getByRole('button', { name: 'Regenerate' }).click();
  await expect(page.getByRole('heading', { name: 'Nicolas Kion' })).toBeVisible();
  expect(Number(psql('select min(id) from combat_events'))).toBeGreaterThan(Number(before));
  expect(psql('select count(*) from combat_events')).toBe('120');
  expect(errors).toEqual([]);
});

test('a log without a raw file says so and offers neither download nor regeneration', async ({
  page,
  request,
  baseURL,
}) => {
  const uuid = await uploadLog(request, baseURL!, TESTLOG);
  deleteRawLog(uuid);
  await page.goto(`/logs/${uuid}`);
  await expect(page.getByTestId('raw-log-unavailable')).toHaveText('No raw log');
  await expect(page.getByRole('link', { name: 'Raw Log' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Regenerate' })).toHaveCount(0);
  expect((await request.get(`/logs/${uuid}/download`)).status()).toBe(404);
});
