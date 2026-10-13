// Shared helpers for the e2e suite: everything talks to the docker stack,
// so the database is seeded and inspected through psql in the
// loganalyzer-postgres container, and the raw logs through the
// loganalyzer-api container.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { expect, type APIRequestContext, type Page } from '@playwright/test';

import type { Upload } from './pages.ts';

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

/** Every table empty, no raw logs. */
export function resetDatabase(): void {
  psql('truncate combat_events, combat_logs, eve_entities restart identity cascade');
  inApi('rm -f /data/combat-logs/*.txt.gz');
}

export function fixture(name: string): Buffer {
  return readFileSync(new URL(`../../tests/fixtures/parser/${name}.txt`, import.meta.url));
}

type CachedName = [kind: 'character' | 'inventory_type', name: string, eveId: number | null];

/** Every name of the larger fixture logs with the id the mock ESI table
 * (tests/fixtures/esi/names.json) has for it, or none. */
const NAMES: Record<string, CachedName[]> = {
  testlog: [
    ['character', 'Aria Vex', 93000001],
    ['character', 'Korr Malan', 93000002],
    ['character', 'Selene Dur', 93000003],
    ['character', 'Tovak Rin', 93000004],
    ['character', 'Logi Pilot', 93000005],
    ['inventory_type', 'Drake', 24698],
    ['inventory_type', 'Hurricane', 24702],
    ['inventory_type', 'Caracal', 621],
    ['inventory_type', 'Rupture', 629],
    ['inventory_type', 'Basilisk', 11985],
  ],
  testlog2: [
    ['character', 'Sabre', null],
    ['inventory_type', 'Sabre', 22456],
    ['character', 'Curse', null],
    ['character', 'Bhaalgorn', null],
    ['character', 'Stiletto', null],
    ['character', 'Ashimmu', null],
    ['inventory_type', 'Curse', null],
    ['inventory_type', 'Bhaalgorn', null],
    ['inventory_type', 'Stiletto', null],
    ['inventory_type', 'Ashimmu', null],
  ],
  testlog3: [
    ['character', 'DarkOldMaker', null],
    ['character', 'Valynda DtAuber', null],
    ['character', 'Daniel Grosny', null],
    ['character', 'Sill Isis', null],
    ['character', 'eeski', null],
    ['character', 'Fardendur', null],
    ['character', 'Jinggels', null],
    ['character', 'Flo May', null],
    ['character', 'Maven Kador', null],
    ['character', 'Shizzles Mantar', null],
    ['character', 'Fogaz', null],
    ['character', 'Vanbo Murgalesh', null],
    ['character', 'vyshnegradsky', null],
    ['character', 'Cab Man', null],
    ['character', 'Ares Tzestu', null],
    ['character', 'Corevas', null],
    ['character', 'Loki', null],
    ['character', 'Salvatore Satriani', null],
    ['character', 'Basilisk', null],
    ['inventory_type', 'Basilisk', 11985],
    ['character', 'Belisarius Koren', null],
    ['character', 'Midgath Stallsman', null],
    ['character', 'GROUND XERO', null],
    ['character', 'Ufoporno', null],
    ['character', 'Okudaa', null],
    ['character', 'Wood3hs Pizza', null],
    ['character', 'Jason Horne', null],
    ['character', 'Devesta Julii', null],
    ['character', 'Anun Naku', null],
    ['character', 'Gorax Skord', null],
    ['character', 'Avion Sundiver', null],
    ['character', 'Calista Seymour', null],
    ['character', 'Lambertx', null],
    ['character', 'Warrior II', null],
    ['character', 'Manfred Stahler', null],
    ['character', 'Kitty Duval', null],
    ['inventory_type', 'Huginn', 11961],
    ['inventory_type', 'Nightmare', null],
    ['inventory_type', 'Claymore', null],
    ['inventory_type', 'Ishkur', null],
    ['inventory_type', 'Ishtar', null],
    ['inventory_type', 'Onyx', null],
    ['inventory_type', 'Deimos', null],
    ['inventory_type', 'Condor', null],
    ['inventory_type', 'Flycatcher', null],
    ['inventory_type', 'Revelation Navy Issue', null],
    ['inventory_type', 'Legion', null],
    ['inventory_type', 'Vengeance', null],
    ['inventory_type', 'Lachesis', null],
    ['inventory_type', 'Broadsword', null],
    ['inventory_type', 'Hurricane Fleet Issue', null],
  ],
};

/**
 * Caches the names of a fixture log before its upload, so the upload asks
 * ESI nothing and the page shows the same ids on every machine.
 */
export function cacheNames(fixtureName: string): void {
  const rows = NAMES[fixtureName];
  if (!rows) {
    return;
  }
  psql(
    `insert into eve_entities (kind, name, eve_id, created_at, updated_at) values ${rows
      .map(
        ([kind, name, eveId]) =>
          `(${quote(kind)}, ${quote(name)}, ${eveId ?? 'null'}, now(), now())`,
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
 * initials show. */
export async function blockEveImages(page: Page): Promise<void> {
  await page.route(/images\.evetech\.net/, (route) => route.abort());
}
