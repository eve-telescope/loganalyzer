// The route contract of the whole stack (Caddy, SvelteKit, Axum): the
// pages, the 404s, the redirects, the paths the api owns and the headers
// every answer carries.
import { expect, test } from '@playwright/test';

import { TESTLOG } from './pages.ts';
import { resetDatabase, uploadLog } from './helpers.ts';

test.beforeEach(() => resetDatabase());

test('serves the two pages', async ({ request, baseURL }) => {
  expect((await request.get('/')).status()).toBe(200);
  const uuid = await uploadLog(request, baseURL!, TESTLOG);
  expect((await request.get(`/logs/${uuid}`)).status()).toBe(200);
  expect((await request.get(`/logs/${uuid.toUpperCase()}`)).status()).toBe(200);
});

test('answers an unknown log with the 404 page', async ({ request }) => {
  for (const path of [
    '/logs/00000000-0000-4000-8000-000000000000',
    '/logs/not-a-uuid',
    '/logs/00000000000040008000000000000000',
    '/nope',
  ]) {
    const response = await request.get(path);
    expect(response.status(), path).toBe(404);
    expect(await response.text(), path).toContain('Not Found');
  }
});

test('redirects trailing slashes', async ({ request, baseURL }) => {
  const uuid = await uploadLog(request, baseURL!, TESTLOG);
  for (const [from, status, to] of [
    [`/logs/${uuid}/`, 308, `/logs/${uuid}`],
    [`/logs/${uuid}/?pilot=Aria%20Vex`, 308, `/logs/${uuid}?pilot=Aria%20Vex`],
    [`/logs/${uuid}/download/`, 308, `/logs/${uuid}/download`],
    ['/up/', 308, '/up'],
  ] as const) {
    const response = await request.get(from, { maxRedirects: 0 });
    expect(response.status(), from).toBe(status);
    const target = new URL(response.headers()['location'], response.url());
    expect(target.pathname + target.search, from).toBe(to);
  }
});

test('keeps the form endpoints in the api', async ({ request }) => {
  const analyze = await request.get('/analyze');
  expect(analyze.status()).toBe(405);
  expect(analyze.headers()['allow']).toBe('POST');
  expect(await analyze.text()).toContain('405 Method Not Allowed');
  expect(
    (await request.get('/logs/00000000-0000-4000-8000-000000000000/regenerate')).status(),
  ).toBe(405);
});

test('serves the health page and the api health check', async ({ request }) => {
  const up = await request.get('/up');
  expect(up.status()).toBe(200);
  expect(await up.text()).toContain('Application up');
  expect(await (await request.get('/api/health')).json()).toEqual({
    status: 'ok',
    database: 'ok',
  });
});

test('serves the icons, the preview image, robots.txt and the manifest', async ({ request }) => {
  for (const name of [
    'favicon.ico',
    'favicon.svg',
    'apple-touch-icon.png',
    'icon-192.png',
    'icon-512.png',
    'icon-512-maskable.png',
    'og-image.png',
    'robots.txt',
    'site.webmanifest',
  ]) {
    const response = await request.get(`/${name}`);
    expect(response.status(), name).toBe(200);
    expect((await response.body()).length, name).toBeGreaterThan(0);
  }
});

test('sends the security headers', async ({ request }) => {
  for (const path of ['/', '/up', '/api/health']) {
    const headers = (await request.get(path)).headers();
    expect(headers['x-frame-options'], path).toBe('SAMEORIGIN');
    expect(headers['x-xss-protection'], path).toBe('1; mode=block');
    expect(headers['x-content-type-options'], path).toBe('nosniff');
  }
});

test('refuses an upload from another site', async ({ request }) => {
  const response = await request.post('/analyze', {
    headers: { Accept: 'application/json', Origin: 'https://evil.example' },
    multipart: {
      log_file: { name: 'combat.txt', mimeType: 'text/plain', buffer: Buffer.from('x') },
    },
  });
  expect(response.status()).toBe(403);
});
