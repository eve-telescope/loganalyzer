import { error } from '@sveltejs/kit';

import { errorMessage } from '#lib/errors.ts';

/**
 * Loads a page's props from the Axum API for an SSR load function. Any
 * status but 200 becomes the error page for that status.
 */
export async function apiGet<T>(fetch: typeof globalThis.fetch, path: string): Promise<T> {
  const response = await fetch(path, { redirect: 'manual' });

  if (!response.ok) {
    error(response.status, errorMessage(response.status));
  }

  return response.json();
}

/** The props endpoint of a page URL: `/api/pages` plus its path, the home
 * page at `/api/pages/home`. */
export function pagesPath(url: URL): string {
  const path = url.pathname === '/' ? '/home' : url.pathname;
  return `/api/pages${path}${url.search}`;
}

/** The load of every page: its props from the api. */
export function loadPage<T>({ fetch, url }: { fetch: typeof globalThis.fetch; url: URL }) {
  return apiGet<T>(fetch, pagesPath(url));
}
