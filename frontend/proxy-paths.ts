// The shared-origin routing contract, mirrored by deploy/Caddyfile: keep
// both in sync when a path changes.

// Paths Axum owns outright: every method proxies to the API. `/api` the
// page props and the health check, `/up` the health page, `/analyze` the
// upload (so a GET gets the api's 405), `/storage` Laravel's old local
// disk route (404).
export const axumPrefixes = ['/api', '/up', '/analyze', '/storage'];

// Below a log: the raw log download and the regeneration
// (`/logs/{uuid}/download`, `/logs/{uuid}/regenerate`).
const axumLogActions = /^\/logs\/[^/]+\/(download|regenerate)$/;

// The forms post to their URLs: every request that is not a GET or HEAD
// goes to Axum as well.
export function isAxumRequest(method: string, path: string): boolean {
  if (method !== 'GET' && method !== 'HEAD') {
    return true;
  }
  return (
    axumPrefixes.some((prefix) => path === prefix || path.startsWith(`${prefix}/`)) ||
    axumLogActions.test(path)
  );
}
