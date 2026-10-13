import { describe, expect, it } from 'vitest';

import { inertiaReload } from './inertia.ts';

describe('inertiaReload', () => {
  it('reloads an Inertia visit in full', () => {
    const url = new URL('https://loganalyzer.eve-telescope.com/logs/x?pilot=A');
    const response = inertiaReload(new Request(url, { headers: { 'X-Inertia': 'true' } }), url);
    expect(response?.status).toBe(409);
    expect(response?.headers.get('X-Inertia-Location')).toBe(url.href);
  });

  it('lets other requests through', () => {
    const url = new URL('https://loganalyzer.eve-telescope.com/');
    expect(inertiaReload(new Request(url), url)).toBeNull();
  });
});
