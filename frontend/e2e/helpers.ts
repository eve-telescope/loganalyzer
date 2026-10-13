// Shared helpers for the e2e suite: everything talks to the docker stack,
// so the database is seeded and inspected through psql in the
// loganalyzer-postgres container, and the raw logs through the
// loganalyzer-api container.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { expect, type APIRequestContext, type Page } from '@playwright/test';

import type { Upload } from './visual-pages.ts';

export function psql(sql: string): string {
  return execFileSync(
    'docker',
    [
      'exec',
      'loganalyzer-postgres',
      'psql',
      '-U',
      'loganalyzer',
      '-d',
      'loganalyzer',
      '-qtAc',
      sql,
    ],
    { encoding: 'utf8' },
  ).trim();
}

/** A shell command in the api container (the raw log volume). */
export function inApi(command: string): string {
  return execFileSync('docker', ['exec', 'loganalyzer-api', 'sh', '-c', command], {
    encoding: 'utf8',
  }).trim();
}

/** A SQL string literal. */
export function quote(value: string | null): string {
  return value === null ? 'null' : `'${value.replaceAll("'", "''")}'`;
}

/** Laravel's RefreshDatabase: every table empty, no raw logs. */
export function resetDatabase(): void {
  psql('truncate combat_events, combat_logs, eve_entities restart identity cascade');
  inApi('rm -f /data/combat-logs/*.txt.gz');
}

export function fixture(name: string): Buffer {
  return readFileSync(new URL(`../../tests/fixtures/parser/${name}.txt`, import.meta.url));
}

const SCENARIOS: Record<string, string> = {
  testlog: 'upload-testlog',
  testlog2: 'upload-testlog2',
  testlog3: 'upload-testlog3',
};

/**
 * The names of a fixture log as the Laravel reference resolved them (its
 * mock ESI), cached before the upload, so the upload asks ESI nothing
 * and the page shows what the reference showed.
 */
export function cacheNames(fixtureName: string): void {
  const scenario = SCENARIOS[fixtureName];
  if (!scenario) {
    return;
  }
  const recorded = JSON.parse(
    readFileSync(new URL('../../tests/fixtures/golden/mutations.json', import.meta.url), 'utf8'),
  ) as {
    name: string;
    rows: { eve_entities: { kind: string; name: string; eve_id: number | null }[] };
  }[];
  const rows = recorded.find((entry) => entry.name === scenario)!.rows.eve_entities;
  if (rows.length === 0) {
    return;
  }
  psql(
    `insert into eve_entities (kind, name, eve_id, created_at, updated_at) values ${rows
      .map(
        (row) => `(${quote(row.kind)}, ${quote(row.name)}, ${row.eve_id ?? 'null'}, now(), now())`,
      )
      .join(', ')} on conflict do nothing`,
  );
}

/** Uploads a fixture log through the api, as the form does; its uuid. */
export async function uploadLog(
  request: APIRequestContext,
  baseURL: string,
  upload: Upload,
): Promise<string> {
  cacheNames(upload.fixture);
  const response = await request.post('/analyze', {
    headers: { Accept: 'application/json', Origin: new URL(baseURL).origin },
    multipart: {
      log_file: { name: upload.fileName, mimeType: 'text/plain', buffer: fixture(upload.fixture) },
    },
  });
  expect(response.status(), await response.text()).toBe(200);
  const { redirect } = (await response.json()) as { redirect: string };
  return redirect.replace('/logs/', '');
}

/** Removes a log's raw file (a log uploaded before raw files were kept). */
export function deleteRawLog(uuid: string): void {
  inApi(`rm -f /data/combat-logs/${uuid}.txt.gz`);
}

/**
 * Collects console errors and page errors. Browsers log every failed
 * fetch as an error; the 422 of a rejected upload is expected.
 */
export function watchErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => {
    if (
      message.type() === 'error' &&
      !message.text().includes('status of 422') &&
      // The portraits and icons the tests block.
      !message.text().includes('net::ERR_FAILED')
    ) {
      errors.push(message.text());
    }
  });
  return errors;
}

/** Portraits and icons stay away (the fixture ids have none), so the
 * initials show, as in the recorded Laravel pages. */
export async function blockEveImages(page: Page): Promise<void> {
  await page.route(/images\.evetech\.net/, (route) => route.abort());
}
