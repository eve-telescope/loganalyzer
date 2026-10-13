// The route contract of the whole stack (Caddy, SvelteKit, Axum) that
// the PHP suite never covered, against the answers the Laravel reference
// gave (tests/fixtures/golden/http and pages).
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';

import { TESTLOG } from './visual-pages.ts';
import { resetDatabase, uploadLog } from './helpers.ts';

test.beforeEach(() => resetDatabase());

function recorded(name: string): {
  status: number;
  sha256?: string;
  headers?: Record<string, string>;
} {
  return JSON.parse(
    readFileSync(new URL(`../../tests/fixtures/golden/http/${name}.json`, import.meta.url), 'utf8'),
  );
}

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

test('redirects trailing slashes and the front controller', async ({ request, baseURL }) => {
  // FIXES.md (deliberate changes): Laravel answered these like the bare path.
  expect(recorded('download-trailing-slash').status).toBe(200);
  const uuid = await uploadLog(request, baseURL!, TESTLOG);
  for (const [from, status, to] of [
    [`/logs/${uuid}/`, 308, `/logs/${uuid}`],
    [`/logs/${uuid}/?pilot=Aria%20Vex`, 308, `/logs/${uuid}?pilot=Aria%20Vex`],
    [`/logs/${uuid}/download/`, 308, `/logs/${uuid}/download`],
    ['/up/', 308, '/up'],
    ['/index.php', 301, '/'],
    [`/index.php/logs/${uuid}`, 301, `/logs/${uuid}`],
  ] as const) {
    const response = await request.get(from, { maxRedirects: 0 });
    expect(response.status(), from).toBe(status);
    const target = new URL(response.headers()['location'], response.url());
    expect(target.pathname + target.search, from).toBe(to);
  }
});

test('keeps the upload and storage paths in the api', async ({ request }) => {
  expect(recorded('analyze-get').status).toBe(405);
  const analyze = await request.get('/analyze');
  expect(analyze.status()).toBe(405);
  expect(analyze.headers()['allow']).toBe('POST');
  expect(await analyze.text()).toContain('405 Method Not Allowed');
  expect((await request.get('/storage/combat-logs/x.txt.gz')).status()).toBe(404);
  expect((await request.put('/storage/x', { data: 'x' })).status()).toBe(404);
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

test('serves the static files of Laravel public/ byte for byte', async ({ request }) => {
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
    const hash = createHash('sha256')
      .update(await response.body())
      .digest('hex');
    expect(hash, name).toBe(recorded(`static-${name}`).sha256);
  }
});

test('sends the security headers nginx added', async ({ request }) => {
  for (const path of ['/', '/up', '/api/health']) {
    const headers = (await request.get(path)).headers();
    expect(headers['x-frame-options'], path).toBe('SAMEORIGIN');
    expect(headers['x-xss-protection'], path).toBe('1; mode=block');
    expect(headers['x-content-type-options'], path).toBe('nosniff');
  }
});

test('reloads tabs left open on the Laravel site', async ({ request, baseURL }) => {
  const uuid = await uploadLog(request, baseURL!, TESTLOG);
  for (const path of ['/', `/logs/${uuid}?pilot=Aria%20Vex`]) {
    const response = await request.get(path, { headers: { 'X-Inertia': 'true' } });
    expect(response.status(), path).toBe(409);
    expect(response.headers()['x-inertia-location'], path).toBe(new URL(path, baseURL).href);
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
