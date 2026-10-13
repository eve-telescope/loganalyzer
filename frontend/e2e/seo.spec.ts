// The <head> of the pages against the head of the Laravel page after
// hydration (scripts/laravel-reference/capture.mjs): the title with the
// app name, the description, OpenGraph and Twitter tags. The analysis
// page's tags come from its events, and link previews read them from
// the server's HTML, so that is checked too.
import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';

import { blockEveImages, resetDatabase, uploadLog } from './helpers.ts';
import { TESTLOG, VISUAL_PAGES, readHead, type Head } from './visual-pages.ts';

const APP_URL = 'https://loganalyzer.eve-telescope.com';

test.beforeEach(() => resetDatabase());

function recorded(name: string): Head {
  return JSON.parse(
    readFileSync(
      new URL(`../../tests/fixtures/golden/heads/${name}.json`, import.meta.url),
      'utf8',
    ),
  );
}

for (const visual of VISUAL_PAGES.filter((page) => page.head)) {
  test(`the head of ${visual.name} is Laravel's`, async ({ page, request, baseURL }) => {
    await blockEveImages(page);
    const uuid = visual.upload ? await uploadLog(request, baseURL!, visual.upload) : '{uuid}';
    await page.goto(visual.path.replace('{uuid}', uuid));
    await page.waitForLoadState('networkidle');
    const head = await page.evaluate(readHead);
    // The stack serves on its own address; production's is APP_URL.
    const ours = JSON.parse(
      JSON.stringify(head).replaceAll(new URL(baseURL!).origin, APP_URL).replaceAll(uuid, '{uuid}'),
    );
    expect(ours).toEqual(recorded(visual.name));
  });
}

test('the server sends the link preview tags in the HTML', async ({ request, baseURL }) => {
  const uuid = await uploadLog(request, baseURL!, TESTLOG);
  const html = await (await request.get(`/logs/${uuid}?pilot=Aria%20Vex`)).text();
  expect(html.match(/<title>/g)).toHaveLength(1);
  expect(html).toContain('<title>Nicolas Kion — Combat Analysis - EVE Log Analyzer</title>');
  expect(html).toContain(
    '<meta property="og:description" content="22,005 damage dealt / 14,445 received — 9 DPS avg — 39m 40s combat"',
  );
  expect(html).toContain(
    `<meta property="og:url" content="${new URL(baseURL!).origin}/logs/${uuid}"`,
  );
  expect(html).toContain('<meta name="twitter:card" content="summary_large_image"');
});
