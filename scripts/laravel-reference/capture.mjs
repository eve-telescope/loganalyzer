// Screenshots, heads and visible text of the Laravel pages, the baselines
// of frontend/e2e/visual.spec.ts, seo.spec.ts and analysis-text.spec.ts:
// the pages of frontend/e2e/visual-pages.ts at desktop and phone width,
// the logs uploaded through the form (names resolved by the mock ESI),
// in UTC and en-US with reduced motion, portraits and icons blocked (the
// mock's ids have none) so the initials show.
//
//   scripts/laravel-reference/run.sh start
//   cd frontend && node ../scripts/laravel-reference/capture.mjs
import { mkdirSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

import { VIEWPORTS, VISUAL_PAGES, readHead, readText } from '../../frontend/e2e/visual-pages.ts';

// Playwright from the frontend's dependencies; nothing is installed here.
const { chromium } = createRequire(new URL('../../frontend/package.json', import.meta.url))(
  'playwright',
);

const BASE = 'http://127.0.0.1:8317';
const APP_URL = 'https://loganalyzer.eve-telescope.com';
const OUT = new URL('../../frontend/e2e/__screenshots__/visual.spec.ts/', import.meta.url);
const HEADS = new URL('../../tests/fixtures/golden/heads/', import.meta.url);
const TEXTS = new URL('../../tests/fixtures/golden/texts/', import.meta.url);
const PARSER = new URL('../../tests/fixtures/parser/', import.meta.url);
mkdirSync(OUT, { recursive: true });
mkdirSync(HEADS, { recursive: true });
mkdirSync(TEXTS, { recursive: true });

const browser = await chromium.launch();
const options = { locale: 'en-US', timezoneId: 'UTC', reducedMotion: 'reduce' };

/** Uploads a fixture through the form; its uuid. */
async function upload(context, { fixture, fileName }) {
  const page = await context.newPage();
  await page.goto(`${BASE}/`);
  const { readFileSync } = await import('node:fs');
  await page.locator('input[type=file]').setInputFiles({
    name: fileName,
    mimeType: 'text/plain',
    buffer: readFileSync(new URL(`${fixture}.txt`, PARSER)),
  });
  await page.getByRole('button', { name: 'ANALYZE COMBAT LOG' }).click();
  await page.waitForURL(/\/logs\//);
  const uuid = new URL(page.url()).pathname.split('/').pop();
  await page.close();
  return uuid;
}

const uploads = new Map();
for (const viewport of VIEWPORTS) {
  const context = await browser.newContext({ ...options, viewport });
  await context.route(/images\.evetech\.net/, (route) => route.abort());
  for (const visual of VISUAL_PAGES) {
    let path = visual.path;
    if (visual.upload) {
      const key = `${visual.upload.fixture}/${visual.upload.fileName}`;
      if (!uploads.has(key)) {
        uploads.set(key, await upload(context, visual.upload));
      }
      path = path.replace('{uuid}', uploads.get(key));
    }
    const page = await context.newPage();
    await page.goto(BASE + path);
    await page.waitForLoadState('networkidle');
    if (visual.click) {
      await page.getByRole('button', { name: visual.click, exact: true }).click();
    }
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(300);
    const file = new URL(`${visual.name}-${viewport.device}.png`, OUT);
    await page.screenshot({ path: file.pathname, animations: 'disabled' });
    if (viewport.device === 'desktop') {
      const uuid = visual.upload ? uploads.get(`${visual.upload.fixture}/${visual.upload.fileName}`) : null;
      const normalize = (value) => {
        let text = JSON.stringify(value).replaceAll(BASE, APP_URL);
        if (uuid) {
          text = text.replaceAll(uuid, '{uuid}');
        }
        return JSON.parse(text);
      };
      if (visual.head) {
        writeFileSync(
          new URL(`${visual.name}.json`, HEADS),
          `${JSON.stringify(normalize(await page.evaluate(readHead)), null, 2)}\n`,
        );
      }
      if (visual.text) {
        writeFileSync(
          new URL(`${visual.name}.json`, TEXTS),
          `${JSON.stringify({ text: normalize(await page.evaluate(readText)) }, null, 2)}\n`,
        );
      }
    }
    await page.close();
    console.log(`captured ${visual.name} (${viewport.device})`);
  }
  await context.close();
}
await browser.close();
