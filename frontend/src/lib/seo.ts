// The <head> of each page as data (rendered by page-head.svelte): the
// title with the app name after it, the description, OpenGraph and
// Twitter tags of the page, and the link preview image every page
// shares.

export interface MetaTag {
  /** `name` or `property`, as the tag was written. */
  attribute: 'name' | 'property';
  key: string;
  content: string;
}

/** The parts of the page URL the head uses. */
export interface PageUrl {
  origin: string;
  pathname: string;
}

export interface HeadData {
  title: string;
  meta: MetaTag[];
}

export const SITE_NAME = 'Combat Log Analyzer';

export const UPLOAD_DESCRIPTION =
  'Upload and analyze EVE Online combat logs. DPS graphs, damage breakdowns, logistics, and detailed pilot statistics.';

const name = (key: string, content: string): MetaTag => ({ attribute: 'name', key, content });
const property = (key: string, content: string): MetaTag => ({
  attribute: 'property',
  key,
  content,
});

/** The tags of every page: the theme color, its URL without the query
 * (the home page without the slash), and the preview image. */
function shared(url: PageUrl): MetaTag[] {
  const image = `${url.origin}/og-image.png`;
  return [
    name('theme-color', '#09090b'),
    property('og:url', `${url.origin}${url.pathname === '/' ? '' : url.pathname}`),
    property('og:image', image),
    property('og:image:width', '1200'),
    property('og:image:height', '630'),
    property('og:image:alt', 'Combat Log Analyzer — EVE Online after-action reports'),
    name('twitter:image', image),
  ];
}

function page(appName: string, title: string, description: string, url: PageUrl): HeadData {
  return {
    title: `${title} - ${appName}`,
    meta: [
      ...shared(url),
      name('description', description),
      property('og:type', 'website'),
      property('og:site_name', SITE_NAME),
      property('og:title', title),
      property('og:description', description),
      name('twitter:card', 'summary_large_image'),
      name('twitter:title', title),
      name('twitter:description', description),
    ],
  };
}

export function uploadHead(appName: string, url: PageUrl): HeadData {
  return page(appName, SITE_NAME, UPLOAD_DESCRIPTION, url);
}

/** The analysis page: the listener in the title, the totals in the
 * description (what Discord and others show for a shared link). */
export function analysisHead(
  appName: string,
  title: string,
  description: string,
  url: PageUrl,
): HeadData {
  return page(appName, title, description, url);
}
