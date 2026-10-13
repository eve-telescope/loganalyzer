import { describe, expect, it } from 'vitest';

import { apiGet, pagesPath } from './api.ts';

describe('pagesPath', () => {
  it('maps a page URL to its props endpoint', () => {
    const base = 'https://loganalyzer.eve-telescope.com';
    expect(pagesPath(new URL('/', base))).toBe('/api/pages/home');
    expect(pagesPath(new URL('/logs/x?pilot=A%20B', base))).toBe('/api/pages/logs/x?pilot=A%20B');
  });
});

describe('apiGet', () => {
  it('answers the props', async () => {
    const fetch = async () => Response.json({ maxUploadSizeMb: 25 });
    expect(await apiGet(fetch as typeof globalThis.fetch, '/api/pages/home')).toEqual({
      maxUploadSizeMb: 25,
    });
  });

  it('turns a failed answer into the error page', async () => {
    const fetch = async () => Response.json({ message: 'Not Found' }, { status: 404 });
    await expect(apiGet(fetch as typeof globalThis.fetch, '/api/pages/logs/x')).rejects.toEqual(
      expect.objectContaining({
        status: 404,
        body: expect.objectContaining({ message: 'Not Found' }),
      }),
    );
  });
});
