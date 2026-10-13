import { describe, expect, it } from 'vitest';

import { canonicalRedirect } from './canonical.ts';

function redirect(path: string): [number, string | null] | null {
  const response = canonicalRedirect(new URL(path, 'https://loganalyzer.eve-telescope.com'));
  return response ? [response.status, response.headers.get('location')] : null;
}

describe('canonicalRedirect', () => {
  it('drops trailing slashes with a 308', () => {
    expect(redirect('/logs/x/')).toEqual([308, '/logs/x']);
    expect(redirect('/logs/x/download/?a=1')).toEqual([308, '/logs/x/download?a=1']);
    expect(redirect('/logs///')).toEqual([308, '/logs']);
  });

  it('leaves the pages alone', () => {
    expect(redirect('/')).toBeNull();
    expect(redirect('/logs/x')).toBeNull();
  });
});
