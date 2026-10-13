import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  eveTimestampToIso,
  formatDateTime,
  formatTime,
  parseTime,
  relativeTimeFromEve,
} from './dates.ts';

afterEach(() => {
  vi.useRealTimers();
});

describe('dates', () => {
  it('reads log times as local times (UTC on the server)', () => {
    const ms = parseTime('2026.02.12 09:41:44');
    expect(ms).toBe(Date.UTC(2026, 1, 12, 9, 41, 44));
    expect(formatDateTime(ms)).toBe('2026-02-12T09:41:44');
    expect(formatTime(ms)).toBe('09:41:44');
  });

  it('turns log times into comparable ISO strings', () => {
    expect(eveTimestampToIso('2026.02.12 09:41:44')).toBe('2026-02-12T09:41:44');
  });

  it('describes the session start relative to now', () => {
    vi.useFakeTimers({ now: Date.UTC(2026, 9, 13, 12, 0, 0) });
    expect(relativeTimeFromEve('2026.10.13 11:59:30')).toBe('just now');
    expect(relativeTimeFromEve('2026.10.13 11:58:00')).toBe('2 minutes ago');
    expect(relativeTimeFromEve('2026.10.12 12:00:00')).toBe('yesterday');
    expect(relativeTimeFromEve('2025.10.13 12:00:00')).toBe('last year');
    expect(relativeTimeFromEve('')).toBeNull();
    expect(relativeTimeFromEve('2026.10.13')).toBeNull();
  });
});
