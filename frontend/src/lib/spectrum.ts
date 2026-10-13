// The hit quality bar (resources/js/components/charts/QualitySpectrum.vue).
// Hit quality is ordinal (Wrecks = best roll, Grazes = worst, Misses =
// none), so each direction uses one hue running bright to dim, with
// misses in neutral slate. Counts are always shown in the legend so
// color never carries the information alone.

export interface QualitySegment {
  label: string;
  count: number;
}

const RAMPS: Record<string, string[]> = {
  cyan: [
    'bg-cyan-200',
    'bg-cyan-300',
    'bg-cyan-400',
    'bg-cyan-500',
    'bg-cyan-600',
    'bg-cyan-700',
    'bg-zinc-600',
  ],
  red: [
    'bg-red-200',
    'bg-red-300',
    'bg-red-400',
    'bg-red-500',
    'bg-red-600',
    'bg-red-700',
    'bg-zinc-600',
  ],
};

export function spectrumTotal(segments: QualitySegment[]): number {
  return segments.reduce((sum, s) => sum + s.count, 0);
}

export function spectrumItems(segments: QualitySegment[], hue: 'cyan' | 'red') {
  const total = spectrumTotal(segments);

  return segments.map((segment, i) => ({
    ...segment,
    colorClass: RAMPS[hue][i] ?? 'bg-zinc-600',
    share: total > 0 ? (segment.count / total) * 100 : 0,
  }));
}

export function visibleSpectrumItems(segments: QualitySegment[], hue: 'cyan' | 'red') {
  return spectrumItems(segments, hue).filter((s) => s.count > 0);
}

export function formatShare(share: number): string {
  return `${share.toFixed(1)}%`;
}
