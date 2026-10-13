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

  it('sends the front controller to the path behind it with a 301', () => {
    expect(redirect('/index.php')).toEqual([301, '/']);
    expect(redirect('/index.php/logs/x?from=a')).toEqual([301, '/logs/x?from=a']);
  });

  it('leaves the pages alone', () => {
    expect(redirect('/')).toBeNull();
    expect(redirect('/logs/x')).toBeNull();
    expect(redirect('/index.phpx')).toBeNull();
  });
});
