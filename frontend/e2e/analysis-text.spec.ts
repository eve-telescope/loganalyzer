// What the analysis page shows, word for word, against the Laravel page
// (scripts/laravel-reference/capture.mjs): every number of the panels,
// the hit quality bars, the tables of each tab and the pilot dialog, for
// the fixture logs with and without a range and a pilot.
import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';

import { blockEveImages, resetDatabase, uploadLog } from './helpers.ts';
import { VISUAL_PAGES, readText } from './visual-pages.ts';

test.use({ reducedMotion: 'reduce' });

test.beforeEach(() => resetDatabase());

function recorded(name: string): string {
  return JSON.parse(
    readFileSync(
      new URL(`../../tests/fixtures/golden/texts/${name}.json`, import.meta.url),
      'utf8',
    ),
  ).text;
}

for (const visual of VISUAL_PAGES.filter((page) => page.text)) {
  test(`${visual.name} reads like Laravel's`, async ({ page, request, baseURL }) => {
    await blockEveImages(page);
    const uuid = await uploadLog(request, baseURL!, visual.upload!);
    await page.goto(visual.path.replace('{uuid}', uuid));
    await page.waitForLoadState('networkidle');
    if (visual.click) {
      await page.getByRole('button', { name: visual.click, exact: true }).click();
    }
    await page.waitForTimeout(300);
    const text = (await page.evaluate(readText)).replaceAll(uuid, '{uuid}');
    expect(text).toBe(recorded(visual.name));
  });
}
