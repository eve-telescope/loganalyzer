// The analysis math against what the Laravel page computed: the legacy
// <script setup> of Analysis.vue run over the reference's page props
// (scripts/laravel-reference/analysis-outputs.mjs, recorded at
// 2026-10-13T12:00:00Z in UTC), one case per log and URL state of
// tests/fixtures/golden/analysis/cases.json.
import { readFileSync } from 'node:fs';
import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  analyze,
  formatDuration,
  ogDescription,
  ogTitle,
  urlQuery,
  initialHiddenSeries,
  initialSelection,
} from './analysis.ts';
import { hueClass, initials, portraitUrl } from './avatar.ts';
import { relativeTimeFromEve } from './dates.ts';
import { visibleSpectrumItems } from './spectrum.ts';
import type { AnalysisProps, Filters } from './types.ts';

const GOLDEN = new URL('../../../tests/fixtures/golden/', import.meta.url);
const RECORDED_AT = Date.UTC(2026, 9, 13, 12, 0, 0);

function json<T>(path: string): T {
  return JSON.parse(readFileSync(new URL(path, GOLDEN), 'utf8')) as T;
}

const cases =
  json<Record<string, { page: string; filters: Partial<Filters> }>>('analysis/cases.json');

/** Sets as the recorder wrote them. */
function plain(value: unknown): unknown {
  return JSON.parse(JSON.stringify(value, (_, v) => (v instanceof Set ? [...v] : v)));
}

afterEach(() => {
  vi.useRealTimers();
});

describe.each(Object.entries(cases))('the %s analysis', (name, { page, filters }) => {
  const recorded = json<Record<string, unknown>>(`analysis/${name}.json`);
  const props = json<{ page: { props: AnalysisProps } }>(`pages/${page}.json`).page.props;
  const merged: Filters = { ...props.filters, ...filters };
  const result = analyze(props.analysis.events, merged);

  it('derives what the Laravel page derived', () => {
    const { filteredEvents, ...derived } = result;
    for (const [key, value] of Object.entries(derived)) {
      expect(plain(value), key).toEqual(recorded[key]);
    }
    expect(filteredEvents.length).toBe(recorded.filteredEventCount);
    expect(merged.pilot).toBe(recorded.selectedPilot);
  });

  it('titles and describes the page like the Laravel page', () => {
    expect(ogTitle(props.analysis.listener)).toBe(recorded.ogTitle);
    expect(ogDescription(result.summary)).toBe(recorded.ogDescription);
  });

  it('formats durations and the session start like the Laravel page', () => {
    vi.useFakeTimers({ now: RECORDED_AT });
    expect([0, 1, 59, 60, 61, 3599, 3600, 7322].map(formatDuration)).toEqual(recorded.durations);
    expect(relativeTimeFromEve(props.analysis.sessionStarted)).toBe(recorded.relativeSessionStart);
  });

  it('draws the avatars and the hit quality bars like the Laravel page', () => {
    const avatars = recorded.avatars as Record<
      string,
      { initials: string; hue: string; portrait: string | null }
    >;
    for (const [pilot, avatar] of Object.entries(avatars)) {
      expect(initials(pilot), pilot).toBe(avatar.initials);
      expect(hueClass(pilot), pilot).toBe(avatar.hue);
      expect(portraitUrl(props.pilotIds[pilot] ?? null), pilot).toBe(avatar.portrait);
    }
    expect(visibleSpectrumItems(result.qualitySegmentsOut, 'cyan')).toEqual(recorded.spectrumOut);
    expect(visibleSpectrumItems(result.qualitySegmentsIn, 'red')).toEqual(recorded.spectrumIn);
  });
});

describe('the URL state', () => {
  const empty: Filters = { from: null, to: null, pilot: null, hide: [] };

  it('selects a range only when both ends are set and differ', () => {
    expect(initialSelection({ ...empty, from: 'a', to: 'b' })).toEqual({ start: 'a', end: 'b' });
    expect(initialSelection({ ...empty, from: 'a', to: 'a' })).toBeNull();
    expect(initialSelection({ ...empty, from: 'a' })).toBeNull();
  });

  it('hides only the six known series', () => {
    expect([...initialHiddenSeries({ ...empty, hide: ['neutIn', 'x', 'dpsDealt'] })]).toEqual([
      'neutIn',
      'dpsDealt',
    ]);
  });

  it('writes the state in a fixed order and skips writing an unchanged URL', () => {
    const query = urlQuery(
      { start: '2026-02-12T10:20:00', end: '2026-02-12T10:25:00' },
      new Set(['neutOut', 'logiDealt']),
      'Aria Vex',
      new URLSearchParams(),
    );
    expect(query).toEqual({
      from: '2026-02-12T10:20:00',
      to: '2026-02-12T10:25:00',
      hide: 'logiDealt,neutOut',
      pilot: 'Aria Vex',
    });
    expect(
      urlQuery(
        { start: '2026-02-12T10:20:00', end: '2026-02-12T10:25:00' },
        new Set(['neutOut', 'logiDealt']),
        'Aria Vex',
        new URLSearchParams(query!),
      ),
    ).toBeNull();
    expect(urlQuery(null, new Set(), null, new URLSearchParams('from=x'))).toEqual({});
  });
});
