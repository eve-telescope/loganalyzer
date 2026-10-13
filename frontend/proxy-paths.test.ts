import { describe, expect, it } from 'vitest';

import { isAxumRequest } from './proxy-paths.ts';

describe('isAxumRequest', () => {
  it('sends every mutation to the api', () => {
    for (const method of ['POST', 'PUT', 'PATCH', 'DELETE']) {
      expect(isAxumRequest(method, '/analyze')).toBe(true);
      expect(isAxumRequest(method, '/logs/x/regenerate')).toBe(true);
    }
  });

  it('sends the api paths to the api', () => {
    for (const path of [
      '/api/pages/home',
      '/api',
      '/up',
      '/analyze',
      '/storage/combat-logs/x.txt.gz',
      '/logs/5a3b6e1c-0d4f-4a2b-8c7d-1e2f3a4b5c01/download',
      '/logs/5a3b6e1c-0d4f-4a2b-8c7d-1e2f3a4b5c01/regenerate',
    ]) {
      expect(isAxumRequest('GET', path), path).toBe(true);
    }
  });

  it('keeps the pages in SvelteKit', () => {
    for (const path of [
      '/',
      '/logs/5a3b6e1c-0d4f-4a2b-8c7d-1e2f3a4b5c01',
      '/logs/x/download/',
      '/favicon.ico',
      '/upload',
      '/apis',
      '/_app/immutable/x.js',
    ]) {
      expect(isAxumRequest('GET', path), path).toBe(false);
    }
  });
});
