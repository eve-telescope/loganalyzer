// The timeline's series and its drag selection
// (resources/js/components/charts/DpsLineChart.vue).
import type { DateTimeRange, DpsDataPoint, SeriesKey } from '#lib/types.ts';

export const SERIES: ReadonlyArray<{
  key: SeriesKey;
  label: string;
  color: string;
  bg: string;
  pick: (d: DpsDataPoint) => number;
}> = [
  {
    key: 'dpsDealt',
    label: 'DPS Dealt',
    color: '#0891b2',
    bg: 'rgba(8, 145, 178, 0.08)',
    pick: (d) => d.dpsDealt,
  },
  {
    key: 'dpsReceived',
    label: 'DPS Received',
    color: '#ef4444',
    bg: 'rgba(239, 68, 68, 0.08)',
    pick: (d) => d.dpsReceived,
  },
  {
    key: 'logiReceived',
    label: 'Logi Received',
    color: '#059669',
    bg: 'rgba(5, 150, 105, 0.08)',
    pick: (d) => d.logiReceived,
  },
  {
    key: 'logiDealt',
    label: 'Logi Dealt',
    color: '#d97706',
    bg: 'rgba(217, 119, 6, 0.08)',
    pick: (d) => d.logiDealt,
  },
  {
    key: 'neutIn',
    label: 'Energy Neut In',
    color: '#8b5cf6',
    bg: 'rgba(139, 92, 246, 0.08)',
    pick: (d) => d.neutIn,
  },
  {
    key: 'neutOut',
    label: 'Energy Neut Out',
    color: '#db2777',
    bg: 'rgba(219, 39, 119, 0.08)',
    pick: (d) => d.neutOut,
  },
];

/** The data point closest to a pixel on the x axis. */
export function indexAtPixel(count: number, pixelOf: (index: number) => number, x: number): number {
  if (count === 0) {
    return 0;
  }

  let closest = 0;
  let closestDist = Infinity;

  for (let i = 0; i < count; i++) {
    const dist = Math.abs(pixelOf(i) - x);

    if (dist < closestDist) {
      closestDist = dist;
      closest = i;
    }
  }

  return closest;
}

/** The range a drag from one data point to another selects; a click
 * (no width) clears the selection. */
export function selectionFromDrag(
  data: DpsDataPoint[],
  from: number,
  to: number,
): DateTimeRange | null {
  const startIndex = Math.min(from, to);
  const endIndex = Math.max(from, to);

  if (startIndex === endIndex || !data[startIndex] || !data[endIndex]) {
    return null;
  }

  return {
    start: data[startIndex].datetime,
    end: data[endIndex].datetime,
  };
}
