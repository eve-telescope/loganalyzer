// The fixture logs the suites upload, and what they read off a page.

export type Upload = { fixture: string; fileName: string };

export const TESTLOG: Upload = { fixture: 'testlog', fileName: 'my-fight.txt' };
export const TESTLOG3: Upload = { fixture: 'testlog3', fileName: 'testlog3.txt' };
export const EMPTY: Upload = { fixture: 'edge-empty', fileName: 'empty.txt' };

/** The parts of the <head> the SEO spec compares. */
export type Head = {
  title: string[];
  meta: Record<string, string>;
  lang: string;
};

/** The <head> as the SEO spec reads it, in the page. */
export function readHead(): Head {
  const meta: Record<string, string> = {};
  for (const element of document.head.querySelectorAll('meta[name], meta[property]')) {
    meta[(element.getAttribute('name') ?? element.getAttribute('property')) as string] =
      element.getAttribute('content') ?? '';
  }
  return {
    title: [...document.querySelectorAll('title')].map((title) => title.textContent ?? ''),
    meta,
    lang: document.documentElement.lang,
  };
}

/** The visible text of the page (the open dialog's too), whitespace
 * collapsed. */
export function readText(): string {
  return document.body.innerText.replace(/\s+/g, ' ').trim();
}
