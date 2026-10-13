/**
 * One URL per page: a path with a trailing slash (`/logs/x/`) redirects
 * to the path without it (308), and the old front controller
 * (`/index.php`, also as a prefix) to the path after it (301). Laravel
 * answered both like the bare path.
 */
export function canonicalRedirect(url: URL): Response | null {
  const { pathname, search } = url;
  let target: string | null = null;
  let status = 308;
  if (pathname === '/index.php' || pathname.startsWith('/index.php/')) {
    target = pathname.slice('/index.php'.length).replace(/\/+$/, '') || '/';
    status = 301;
  } else if (pathname.length > 1 && pathname.endsWith('/')) {
    target = pathname.replace(/\/+$/, '') || '/';
  }
  if (target === null) {
    return null;
  }
  return new Response(null, { status, headers: { location: `${target}${search}` } });
}
