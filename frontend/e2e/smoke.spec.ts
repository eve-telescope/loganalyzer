// Every page loads without a JavaScript error, on the server and after
// hydration.
import { expect, test } from '@playwright/test';

import { blockEveImages, resetDatabase, uploadLog, watchErrors } from './helpers.ts';
import { EMPTY, TESTLOG, TESTLOG3 } from './pages.ts';

test.beforeEach(() => resetDatabase());

test('no page logs an error', async ({ page, request, baseURL }) => {
  const errors = watchErrors(page);
  await blockEveImages(page);
  const paths = ['/', '/logs/00000000-0000-4000-8000-000000000000'];
  for (const upload of [TESTLOG, TESTLOG3, EMPTY]) {
    const uuid = await uploadLog(request, baseURL!, upload);
    paths.push(`/logs/${uuid}`, `/logs/${uuid}?from=x&to=y&hide=bogus&pilot=Nobody`);
  }
  for (const path of paths) {
    await page.goto(path);
    await page.waitForLoadState('networkidle');
  }
  // The 404 of the unknown log is the one expected failed load.
  expect(errors.filter((error) => !error.includes('404'))).toEqual([]);
});
