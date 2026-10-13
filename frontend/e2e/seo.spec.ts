// The <head> of the pages after hydration: the title with the app name,
// the description, OpenGraph and Twitter tags. The analysis page's tags
// come from its events, and link previews read them from the server's
// HTML, so that is checked too.
import { expect, test } from '@playwright/test';

import { blockEveImages, resetDatabase, uploadLog } from './helpers.ts';
import { EMPTY, TESTLOG, TESTLOG3, readHead, type Head, type Upload } from './pages.ts';

const APP_URL = 'https://loganalyzer.eve-telescope.com';
const IMAGE = `${APP_URL}/og-image.png`;

test.beforeEach(() => resetDatabase());

/** The head of a page with its own title, description and URL. */
function head(title: string, description: string, url: string): Head {
  return {
    title: [`${title} - EVE Log Analyzer`],
    meta: {
      viewport: 'width=device-width, initial-scale=1',
      'theme-color': '#09090b',
      'og:url': url,
      'og:image': IMAGE,
      'og:image:width': '1200',
      'og:image:height': '630',
      'og:image:alt': 'Combat Log Analyzer — EVE Online after-action reports',
      'twitter:image': IMAGE,
      description,
      'og:type': 'website',
      'og:site_name': 'Combat Log Analyzer',
      'og:title': title,
      'og:description': description,
      'twitter:card': 'summary_large_image',
      'twitter:title': title,
      'twitter:description': description,
    },
    lang: 'en',
  };
}

const LOG_URL = `${APP_URL}/logs/{uuid}`;
const TESTLOG_TOTALS = '22,005 damage dealt / 14,445 received — 9 DPS avg — 39m 40s combat';

const PAGES: { name: string; upload?: Upload; path: string; head: Head }[] = [
  {
    name: 'the upload page',
    path: '/',
    head: head(
      'Combat Log Analyzer',
      'Upload and analyze EVE Online combat logs. DPS graphs, damage breakdowns, logistics, and detailed pilot statistics.',
      APP_URL,
    ),
  },
  {
    name: 'an analysis',
    upload: TESTLOG,
    path: '/logs/{uuid}',
    head: head('Nicolas Kion — Combat Analysis', TESTLOG_TOTALS, LOG_URL),
  },
  {
    name: 'an analysis of a range',
    upload: TESTLOG,
    path: '/logs/{uuid}?from=2026-02-12T10:20:00&to=2026-02-12T10:25:00&hide=logiDealt,neutOut',
    head: head(
      'Nicolas Kion — Combat Analysis',
      '1,983 damage dealt / 1,507 received — 14 DPS avg — 2m 20s combat',
      LOG_URL,
    ),
  },
  {
    name: 'an analysis with a pilot open',
    upload: TESTLOG,
    path: '/logs/{uuid}?pilot=Aria%20Vex',
    head: head('Nicolas Kion — Combat Analysis', TESTLOG_TOTALS, LOG_URL),
  },
  {
    name: 'a fleet fight',
    upload: TESTLOG3,
    path: '/logs/{uuid}',
    head: head(
      'De BuG — Combat Analysis',
      '17,115 damage dealt / 202,827 received — 28 DPS avg — 10m 13s combat',
      LOG_URL,
    ),
  },
  {
    name: 'an empty log',
    upload: EMPTY,
    path: '/logs/{uuid}',
    head: head(
      'Unknown — Combat Analysis',
      '0 damage dealt / 0 received — 0 DPS avg — 0s combat',
      LOG_URL,
    ),
  },
  {
    name: 'an unknown log',
    path: '/logs/00000000-0000-4000-8000-000000000000',
    head: {
      title: ['Not Found'],
      meta: { viewport: 'width=device-width, initial-scale=1' },
      lang: 'en',
    },
  },
];

for (const visit of PAGES) {
  test(`the head of ${visit.name}`, async ({ page, request, baseURL }) => {
    await blockEveImages(page);
    const uuid = visit.upload ? await uploadLog(request, baseURL!, visit.upload) : '{uuid}';
    await page.goto(visit.path.replace('{uuid}', uuid));
    await page.waitForLoadState('networkidle');
    const read = await page.evaluate(readHead);
    // The stack serves on its own address; production's is APP_URL.
    const ours = JSON.parse(
      JSON.stringify(read).replaceAll(new URL(baseURL!).origin, APP_URL).replaceAll(uuid, '{uuid}'),
    );
    expect(ours).toEqual(visit.head);
  });
}

test('the server sends the link preview tags in the HTML', async ({ request, baseURL }) => {
  const uuid = await uploadLog(request, baseURL!, TESTLOG);
  const html = await (await request.get(`/logs/${uuid}?pilot=Aria%20Vex`)).text();
  expect(html.match(/<title>/g)).toHaveLength(1);
  expect(html).toContain('<title>Nicolas Kion — Combat Analysis - EVE Log Analyzer</title>');
  expect(html).toContain(`<meta property="og:description" content="${TESTLOG_TOTALS}"`);
  expect(html).toContain(
    `<meta property="og:url" content="${new URL(baseURL!).origin}/logs/${uuid}"`,
  );
  expect(html).toContain('<meta name="twitter:card" content="summary_large_image"');
});
