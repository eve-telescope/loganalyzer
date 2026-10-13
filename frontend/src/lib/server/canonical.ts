/**
 * One URL per page: a path with a trailing slash (`/logs/x/`) redirects
 * to the path without it (308).
 */
export function canonicalRedirect(url: URL): Response | null {
  const { pathname, search } = url;
  if (pathname.length <= 1 || !pathname.endsWith('/')) {
    return null;
  }
  const target = pathname.replace(/\/+$/, '') || '/';
  return new Response(null, { status: 308, headers: { location: `${target}${search}` } });
}
