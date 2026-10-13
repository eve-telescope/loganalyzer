// Every page against the screenshot of the Laravel page
// (scripts/laravel-reference/capture.mjs), at desktop and phone width,
// within the 3% pixel tolerance of playwright.config.ts. The baselines
// depend on the host's fonts and the day they were taken (the session
// start reads "N months ago"), so CI skips them.
import { expect, test } from '@playwright/test';

import { blockEveImages, resetDatabase, uploadLog } from './helpers.ts';
import { VIEWPORTS, VISUAL_PAGES } from './visual-pages.ts';

test.skip(Boolean(process.env.CI), 'screenshots are host specific');

test.use({ reducedMotion: 'reduce' });

test.beforeAll(() => resetDatabase());

const uploads = new Map<string, string>();

for (const visual of VISUAL_PAGES) {
  for (const viewport of VIEWPORTS) {
    test(`${visual.name} looks like Laravel's (${viewport.device})`, async ({
      page,
      request,
      baseURL,
    }) => {
      await blockEveImages(page);
      let path = visual.path;
      if (visual.upload) {
        const key = `${visual.upload.fixture}/${visual.upload.fileName}`;
        if (!uploads.has(key)) {
          uploads.set(key, await uploadLog(request, baseURL!, visual.upload));
        }
        path = path.replace('{uuid}', uploads.get(key)!);
      }
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto(path);
      await page.waitForLoadState('networkidle');
      if (visual.click) {
        await page.getByRole('button', { name: visual.click, exact: true }).click();
      }
      await page.evaluate(() => document.fonts.ready);
      await page.waitForTimeout(300);
      await expect(page).toHaveScreenshot(`${visual.name}-${viewport.device}.png`, {
        fullPage: false,
      });
    });
  }
}
