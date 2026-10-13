// The pages the visual comparison shoots, shared by the visual spec and
// scripts/laravel-reference/capture.mjs so both sides take the same
// pictures: the fixture logs uploaded through the form, the same names
// resolved. `head` marks the pages whose <head> is recorded, `text` the
// ones whose visible text is.

export type Upload = { fixture: string; fileName: string };

export type VisualPage = {
  name: string;
  /** The log to upload first; `{uuid}` in the path is its uuid. */
  upload?: Upload;
  path: string;
  /** A button to click once the page is there (a tab). */
  click?: string;
  head?: boolean;
  text?: boolean;
};

export const TESTLOG: Upload = { fixture: 'testlog', fileName: 'my-fight.txt' };
export const TESTLOG3: Upload = { fixture: 'testlog3', fileName: 'testlog3.txt' };
export const EMPTY: Upload = { fixture: 'edge-empty', fileName: 'empty.txt' };

export const VISUAL_PAGES: VisualPage[] = [
  { name: 'home', path: '/', head: true },
  { name: 'analysis', upload: TESTLOG, path: '/logs/{uuid}', head: true, text: true },
  {
    name: 'analysis-range',
    upload: TESTLOG,
    path: '/logs/{uuid}?from=2026-02-12T10:20:00&to=2026-02-12T10:25:00&hide=logiDealt,neutOut',
    head: true,
    text: true,
  },
  {
    name: 'analysis-pilot',
    upload: TESTLOG,
    path: '/logs/{uuid}?pilot=Aria%20Vex',
    head: true,
    text: true,
  },
  {
    name: 'analysis-incoming',
    upload: TESTLOG,
    path: '/logs/{uuid}',
    click: 'Incoming',
    text: true,
  },
  {
    name: 'analysis-logistics',
    upload: TESTLOG,
    path: '/logs/{uuid}',
    click: 'Logistics',
    text: true,
  },
  { name: 'analysis-fleet', upload: TESTLOG3, path: '/logs/{uuid}', head: true, text: true },
  {
    name: 'analysis-fleet-range',
    upload: TESTLOG3,
    path: '/logs/{uuid}?from=2026-05-28T19:12:00&to=2026-05-28T19:15:30&pilot=DarkOldMaker',
    text: true,
  },
  { name: 'analysis-empty', upload: EMPTY, path: '/logs/{uuid}', head: true, text: true },
  { name: 'not-found', path: '/logs/00000000-0000-4000-8000-000000000000', head: true },
];

export const VIEWPORTS = [
  { device: 'desktop', width: 1280, height: 800 },
  { device: 'phone', width: 390, height: 844 },
] as const;

/** The parts of the <head> the SEO spec compares. */
export type Head = {
  title: string[];
  meta: Record<string, string>;
  lang: string;
};

/** What the SEO spec reads, in the page. */
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
