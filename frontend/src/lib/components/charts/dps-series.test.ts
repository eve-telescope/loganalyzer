import { describe, expect, it } from 'vitest';

import type { DpsDataPoint } from '#lib/types.ts';

import { SERIES, indexAtPixel, selectionFromDrag } from './dps-series.ts';

const point = (datetime: string): DpsDataPoint => ({
  datetime,
  label: datetime.slice(11),
  dpsDealt: 1,
  dpsReceived: 2,
  logiReceived: 3,
  logiDealt: 4,
  neutIn: 5,
  neutOut: 6,
});

describe('the timeline', () => {
  it('has the six series in the legend order of the URL keys', () => {
    expect(SERIES.map((s) => s.key)).toEqual([
      'dpsDealt',
      'dpsReceived',
      'logiReceived',
      'logiDealt',
      'neutIn',
      'neutOut',
    ]);
    expect(SERIES.map((s) => s.pick(point('2026-01-01T00:00:00')))).toEqual([1, 2, 3, 4, 5, 6]);
  });

  it('finds the data point closest to the pointer', () => {
    expect(indexAtPixel(5, (i) => i * 10, 23)).toBe(2);
    expect(indexAtPixel(5, (i) => i * 10, 26)).toBe(3);
    expect(indexAtPixel(0, (i) => i, 3)).toBe(0);
  });

  it('selects the dragged range in either direction, and a click clears it', () => {
    const data = ['00:00:00', '00:00:05', '00:00:10'].map((t) => point(`2026-01-01T${t}`));
    expect(selectionFromDrag(data, 2, 0)).toEqual({
      start: '2026-01-01T00:00:00',
      end: '2026-01-01T00:00:10',
    });
    expect(selectionFromDrag(data, 1, 1)).toBeNull();
    expect(selectionFromDrag(data, 1, 9)).toBeNull();
  });
});
