// Everything the analysis page shows, derived from the event list: the
// functions of the Laravel page (resources/js/pages/Analysis.vue) as they
// were, each taking what its computed property read.

import { eveTimestampToIso, formatDateTime, formatTime, parseTime } from './dates.ts';
import type {
  CombatEventData,
  DateTimeRange,
  DpsDataPoint,
  Filters,
  SeriesKey,
  TargetDamage,
  WeaponDamage,
} from './types.ts';

export const ALL_SERIES_KEYS: ReadonlyArray<SeriesKey> = [
  'dpsDealt',
  'dpsReceived',
  'logiReceived',
  'logiDealt',
  'neutIn',
  'neutOut',
];

const VALID_SERIES_KEYS = new Set<SeriesKey>(ALL_SERIES_KEYS);

const BUCKET_SECONDS = 5;

/** EVE hit qualities from best roll to worst, misses last. */
export const QUALITY_ORDER = [
  'Wrecks',
  'Smashes',
  'Penetrates',
  'Hits',
  'Glances Off',
  'Grazes',
  'Misses',
] as const;

const PEAK_WINDOW_SECONDS = 10;

export const ENGAGEMENT_TYPES = ['damage', 'logistics', 'neutralization'] as const;
export type EngagementType = (typeof ENGAGEMENT_TYPES)[number];
export type EngagementDirection = 'both' | 'outgoing' | 'incoming';

export const ENGAGEMENT_TYPE_LABELS: Record<EngagementType, string> = {
  damage: 'Damage',
  logistics: 'Logistics',
  neutralization: 'Energy',
};

/** The range a shared link selects: only when both ends are set and differ. */
export function initialSelection(filters: Filters): DateTimeRange | null {
  return filters.from && filters.to && filters.from !== filters.to
    ? { start: filters.from, end: filters.to }
    : null;
}

/** The series a shared link hides, the six known keys only. */
export function initialHiddenSeries(filters: Filters): Set<SeriesKey> {
  return new Set(filters.hide.filter((k): k is SeriesKey => VALID_SERIES_KEYS.has(k as SeriesKey)));
}

export function ogTitle(listener: string): string {
  return `${listener} — Combat Analysis`;
}

export function ogDescription(summary: Summary): string {
  return `${formatNumber(summary.totalDamageDealt)} damage dealt / ${formatNumber(summary.totalDamageReceived)} received — ${formatNumber(Math.round(summary.dpsDealt))} DPS avg — ${formatDuration(summary.combatDurationSeconds)} combat`;
}

export function dpsOverTime(events: CombatEventData[]): DpsDataPoint[] {
  if (events.length === 0) {
    return [];
  }

  const firstMs = parseTime(events[0].timestamp);
  const lastMs = parseTime(events[events.length - 1].timestamp);
  const totalSeconds = Math.round((lastMs - firstMs) / 1000);
  const bucketCount = Math.floor(totalSeconds / BUCKET_SECONDS) + 1;

  const buckets: {
    dealt: number;
    received: number;
    logiIn: number;
    logiOut: number;
    neutIn: number;
    neutOut: number;
    datetime: string;
    label: string;
  }[] = [];

  for (let i = 0; i < bucketCount; i++) {
    const bucketMs = firstMs + i * BUCKET_SECONDS * 1000;

    buckets.push({
      dealt: 0,
      received: 0,
      logiIn: 0,
      logiOut: 0,
      neutIn: 0,
      neutOut: 0,
      datetime: formatDateTime(bucketMs),
      label: formatTime(bucketMs),
    });
  }

  for (const e of events) {
    const ms = parseTime(e.timestamp);
    const idx = Math.min(Math.floor((ms - firstMs) / 1000 / BUCKET_SECONDS), bucketCount - 1);

    if (e.type === 'logistics') {
      if (e.direction === 'incoming') {
        buckets[idx].logiIn += e.damage;
      } else {
        buckets[idx].logiOut += e.damage;
      }
    } else if (e.type === 'neutralization') {
      if (e.direction === 'incoming') {
        buckets[idx].neutIn += e.damage;
      } else {
        buckets[idx].neutOut += e.damage;
      }
    } else if (e.direction === 'outgoing') {
      buckets[idx].dealt += e.damage;
    } else {
      buckets[idx].received += e.damage;
    }
  }

  return buckets.map((b) => ({
    datetime: b.datetime,
    label: b.label,
    dpsDealt: Math.round((b.dealt / BUCKET_SECONDS) * 100) / 100,
    dpsReceived: Math.round((b.received / BUCKET_SECONDS) * 100) / 100,
    logiReceived: Math.round((b.logiIn / BUCKET_SECONDS) * 100) / 100,
    logiDealt: Math.round((b.logiOut / BUCKET_SECONDS) * 100) / 100,
    neutIn: Math.round((b.neutIn / BUCKET_SECONDS) * 100) / 100,
    neutOut: Math.round((b.neutOut / BUCKET_SECONDS) * 100) / 100,
  }));
}

export function filterEvents(
  events: CombatEventData[],
  selection: DateTimeRange | null,
): CombatEventData[] {
  if (!selection) {
    return events;
  }

  const { start, end } = selection;

  return events.filter((e) => {
    const dt = eveTimestampToIso(e.timestamp);

    return dt >= start && dt <= end;
  });
}

interface BiggestHit {
  damage: number;
  weapon: string;
  pilot: string;
}

export function buildDetails(events: CombatEventData[]) {
  const qualityOut: Record<string, number> = {};
  const qualityIn: Record<string, number> = {};
  const targets = new Set<string>();
  const attackers = new Set<string>();
  const pilots = new Set<string>();
  let biggestHitOut: BiggestHit | null = null;
  let biggestHitIn: BiggestHit | null = null;
  let neutIn = 0;
  let neutOut = 0;
  let neutInEvents = 0;
  let neutOutEvents = 0;
  let logiInCycles = 0;
  let logiOutCycles = 0;

  for (const e of events) {
    pilots.add(e.playerName);

    if (e.type === 'damage') {
      if (e.direction === 'outgoing') {
        targets.add(e.playerName);
        qualityOut[e.quality] = (qualityOut[e.quality] ?? 0) + 1;

        if (e.damage > (biggestHitOut?.damage ?? 0)) {
          biggestHitOut = {
            damage: e.damage,
            weapon: e.weapon,
            pilot: e.playerName,
          };
        }
      } else {
        attackers.add(e.playerName);
        qualityIn[e.quality] = (qualityIn[e.quality] ?? 0) + 1;

        if (e.damage > (biggestHitIn?.damage ?? 0)) {
          biggestHitIn = {
            damage: e.damage,
            weapon: e.weapon,
            pilot: e.playerName,
          };
        }
      }
    } else if (e.type === 'neutralization') {
      if (e.direction === 'incoming') {
        neutIn += e.damage;
        neutInEvents++;
      } else {
        neutOut += e.damage;
        neutOutEvents++;
      }
    } else if (e.type === 'logistics') {
      if (e.direction === 'incoming') {
        logiInCycles++;
      } else {
        logiOutCycles++;
      }
    }
  }

  return {
    qualityOut,
    qualityIn,
    biggestHitOut,
    biggestHitIn,
    uniqueTargets: targets.size,
    uniqueAttackers: attackers.size,
    pilotsInvolved: pilots.size,
    neutIn,
    neutOut,
    neutInEvents,
    neutOutEvents,
    logiInCycles,
    logiOutCycles,
  };
}

export type Details = ReturnType<typeof buildDetails>;

export function peakDps(events: CombatEventData[]): { dealt: number; received: number } {
  if (events.length === 0) {
    return { dealt: 0, received: 0 };
  }

  const firstMs = parseTime(events[0].timestamp);
  const dealtBuckets = new Map<number, number>();
  const receivedBuckets = new Map<number, number>();

  for (const e of events) {
    if (e.type !== 'damage') {
      continue;
    }

    const idx = Math.floor((parseTime(e.timestamp) - firstMs) / 1000 / PEAK_WINDOW_SECONDS);
    const buckets = e.direction === 'outgoing' ? dealtBuckets : receivedBuckets;

    buckets.set(idx, (buckets.get(idx) ?? 0) + e.damage);
  }

  const peakOf = (buckets: Map<number, number>) =>
    buckets.size > 0 ? Math.round(Math.max(...buckets.values()) / PEAK_WINDOW_SECONDS) : 0;

  return { dealt: peakOf(dealtBuckets), received: peakOf(receivedBuckets) };
}

/** HP repaired onto the listener per HP of damage taken. */
export function tankSupportRatio(summary: Summary): number | string {
  if (summary.totalDamageReceived === 0) {
    return '—';
  }

  return (summary.totalLogiReceived / summary.totalDamageReceived) * 100;
}

export function neutPressureFor(summary: Summary, totalGj: number): number | string {
  const duration = summary.combatDurationSeconds;

  return duration === 0 || totalGj === 0 ? '—' : totalGj / duration;
}

export function qualitySegments(quality: Record<string, number>) {
  return QUALITY_ORDER.map((label) => ({
    label,
    count: quality[label] ?? 0,
  }));
}

export function buildSummary(events: CombatEventData[]) {
  let totalDamageDealt = 0;
  let totalDamageReceived = 0;
  let outgoingHits = 0;
  let outgoingMisses = 0;
  let incomingHits = 0;
  let incomingMisses = 0;
  let logiReceived = 0;
  let logiDealt = 0;

  for (const e of events) {
    if (e.type === 'logistics') {
      if (e.direction === 'incoming') {
        logiReceived += e.damage;
      } else {
        logiDealt += e.damage;
      }

      continue;
    }

    if (e.type !== 'damage') {
      continue;
    }

    if (e.direction === 'outgoing') {
      totalDamageDealt += e.damage;

      if (e.quality === 'Misses') {
        outgoingMisses++;
      } else {
        outgoingHits++;
      }
    } else {
      totalDamageReceived += e.damage;

      if (e.quality === 'Misses') {
        incomingMisses++;
      } else {
        incomingHits++;
      }
    }
  }

  const duration = getDurationSeconds(events);

  return {
    totalDamageDealt,
    totalDamageReceived,
    combatDurationSeconds: duration,
    dpsDealt: duration > 0 ? Math.round((totalDamageDealt / duration) * 100) / 100 : 0,
    dpsReceived: duration > 0 ? Math.round((totalDamageReceived / duration) * 100) / 100 : 0,
    totalOutgoingHits: outgoingHits,
    totalOutgoingMisses: outgoingMisses,
    totalIncomingHits: incomingHits,
    totalIncomingMisses: incomingMisses,
    totalLogiReceived: logiReceived,
    totalLogiDealt: logiDealt,
  };
}

export type Summary = ReturnType<typeof buildSummary>;

function getDurationSeconds(events: CombatEventData[]): number {
  if (events.length < 2) {
    return 0;
  }

  const first = parseTime(events[0].timestamp);
  const last = parseTime(events[events.length - 1].timestamp);

  return Math.round((last - first) / 1000);
}

export function buildDamageByEntity(
  events: CombatEventData[],
  direction: 'outgoing' | 'incoming',
): Record<string, TargetDamage> {
  const result: Record<string, TargetDamage> = {};

  for (const e of events) {
    if (e.direction !== direction || e.type !== 'damage') {
      continue;
    }

    if (!result[e.playerName]) {
      result[e.playerName] = {
        damage: 0,
        hits: 0,
        misses: 0,
        ship: e.shipName,
      };
    }

    result[e.playerName].damage += e.damage;

    if (e.quality === 'Misses') {
      result[e.playerName].misses++;
    } else {
      result[e.playerName].hits++;
    }

    if (e.shipName) {
      result[e.playerName].ship = e.shipName;
    }
  }

  return Object.fromEntries(Object.entries(result).sort(([, a], [, b]) => b.damage - a.damage));
}

export function buildWeaponBreakdown(
  events: CombatEventData[],
  direction: 'outgoing' | 'incoming',
): Record<string, WeaponDamage> {
  const result: Record<string, WeaponDamage> = {};

  for (const e of events) {
    if (e.direction !== direction || e.type !== 'damage') {
      continue;
    }

    if (!result[e.weapon]) {
      result[e.weapon] = { damage: 0, hits: 0 };
    }

    result[e.weapon].damage += e.damage;

    if (e.quality !== 'Misses') {
      result[e.weapon].hits++;
    }
  }

  return Object.fromEntries(Object.entries(result).sort(([, a], [, b]) => b.damage - a.damage));
}

/** The rows of the outgoing and incoming tables. */
export function pilotTableRows(byEntity: Record<string, TargetDamage>, summary: Summary) {
  return Object.entries(byEntity).map(([name, data]) => {
    const dur = summary.combatDurationSeconds;

    return {
      name,
      ship: data.ship ?? '-',
      damage: data.damage,
      dps: dur > 0 ? Math.round(data.damage / dur) : 0,
      hits: data.hits,
      misses: data.misses,
      hitRate:
        data.hits + data.misses > 0
          ? `${((data.hits / (data.hits + data.misses)) * 100).toFixed(1)}%`
          : '-',
    };
  });
}

export function logiTableRows(events: CombatEventData[]) {
  const logiEvents = events.filter((e) => e.type === 'logistics');
  const bySource: Record<
    string,
    { amount: number; count: number; ship: string | null; module: string }
  > = {};

  for (const e of logiEvents) {
    const key = `${e.playerName}|${e.direction}`;

    if (!bySource[key]) {
      bySource[key] = {
        amount: 0,
        count: 0,
        ship: e.shipName,
        module: e.weapon,
      };
    }

    bySource[key].amount += e.damage;
    bySource[key].count++;

    if (e.shipName) {
      bySource[key].ship = e.shipName;
    }
  }

  return Object.entries(bySource)
    .map(([key, data]) => {
      const [name, direction] = key.split('|');

      return {
        name,
        direction: direction === 'incoming' ? 'Received' : 'Dealt',
        ship: data.ship ?? '-',
        module: data.module,
        amount: data.amount,
        count: data.count,
      };
    })
    .sort((a, b) => b.amount - a.amount);
}

/** The bars of the by-target and by-source charts. */
export function entityItems(byEntity: Record<string, TargetDamage>) {
  return Object.entries(byEntity)
    .sort(([, a], [, b]) => b.damage - a.damage)
    .map(([name, data]) => ({
      label: data.ship ? `${name} (${data.ship})` : name,
      value: data.damage,
    }));
}

/** The slices of the by-weapon charts. */
export function weaponItems(byWeapon: Record<string, WeaponDamage>) {
  return Object.entries(byWeapon)
    .sort(([, a], [, b]) => b.damage - a.damage)
    .map(([name, data]) => ({
      label: name,
      value: data.damage,
    }));
}

export function selectionLabel(selection: DateTimeRange | null): string | null {
  if (!selection) {
    return null;
  }

  const start = selection.start.split('T')[1] ?? selection.start;
  const end = selection.end.split('T')[1] ?? selection.end;

  return `${start} - ${end}`;
}

export function formatDuration(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;

  return mins > 0 ? `${mins}m ${secs}s` : `${secs}s`;
}

export function formatNumber(n: number): string {
  return n.toLocaleString();
}

export function percentFormat(n: number): string {
  return `${n.toFixed(1)}%`;
}

export function wholePercentFormat(n: number): string {
  return `${Math.round(n)}%`;
}

export function gjFormat(n: number): string {
  return `${Math.round(n).toLocaleString()} GJ`;
}

export function gjPerSecondFormat(n: number): string {
  return `${n.toFixed(1)} GJ/s`;
}

export function accuracyValue(hits: number, misses: number): number | string {
  const total = hits + misses;

  return total > 0 ? (hits / total) * 100 : '—';
}

export function pilotEvents(
  events: CombatEventData[],
  selectedPilot: string | null,
): CombatEventData[] {
  return selectedPilot ? events.filter((e) => e.playerName === selectedPilot) : [];
}

export function engagementOf(events: CombatEventData[]) {
  let dealt = 0;
  let received = 0;
  let hitsOn = 0;
  let missesOn = 0;
  let hitsBy = 0;
  let missesBy = 0;
  let logiIn = 0;
  let logiOut = 0;
  let neutIn = 0;
  let neutOut = 0;
  let ship: string | null = null;

  for (const e of events) {
    if (e.shipName) {
      ship = e.shipName;
    }

    if (e.type === 'damage') {
      if (e.direction === 'outgoing') {
        dealt += e.damage;

        if (e.quality === 'Misses') {
          missesOn++;
        } else {
          hitsOn++;
        }
      } else {
        received += e.damage;

        if (e.quality === 'Misses') {
          missesBy++;
        } else {
          hitsBy++;
        }
      }
    } else if (e.type === 'logistics') {
      if (e.direction === 'incoming') {
        logiIn += e.damage;
      } else {
        logiOut += e.damage;
      }
    } else if (e.direction === 'incoming') {
      neutIn += e.damage;
    } else {
      neutOut += e.damage;
    }
  }

  return {
    dealt,
    received,
    hitsOn,
    missesOn,
    hitsBy,
    missesBy,
    logiIn,
    logiOut,
    neutIn,
    neutOut,
    ship,
  };
}

export function engagementLog(
  events: CombatEventData[],
  types: Set<EngagementType>,
  direction: EngagementDirection,
) {
  return events
    .filter((e) => types.has(e.type))
    .filter((e) => direction === 'both' || e.direction === direction)
    .map((e) => Object.assign({}, e, { time: e.timestamp.split(' ')[1] ?? e.timestamp }));
}

export function eventColorClass(e: { type: string; direction: string }): string {
  if (e.type === 'damage') {
    return e.direction === 'outgoing' ? 'text-cyan-400' : 'text-red-400';
  }

  if (e.type === 'logistics') {
    return e.direction === 'incoming' ? 'text-emerald-400' : 'text-amber-400';
  }

  return e.direction === 'incoming' ? 'text-violet-400' : 'text-pink-400';
}

/**
 * The URL state of the page: the selected range, the hidden series in
 * their fixed order and the open pilot. `null` when the current query
 * already says the same.
 */
export function urlQuery(
  selection: DateTimeRange | null,
  hidden: Set<SeriesKey>,
  pilot: string | null,
  current: URLSearchParams,
): Record<string, string> | null {
  const query: Record<string, string> = {};

  if (selection) {
    query.from = selection.start;
    query.to = selection.end;
  }

  if (hidden.size > 0) {
    query.hide = ALL_SERIES_KEYS.filter((k) => hidden.has(k)).join(',');
  }

  if (pilot) {
    query.pilot = pilot;
  }

  if (
    current.get('from') === (query.from ?? null) &&
    current.get('to') === (query.to ?? null) &&
    current.get('hide') === (query.hide ?? null) &&
    current.get('pilot') === (query.pilot ?? null)
  ) {
    return null;
  }

  return query;
}

/** Everything the page derives at once, for the characterization tests. */
export function analyze(events: CombatEventData[], filters: Filters) {
  const selection = initialSelection(filters);
  const filtered = filterEvents(events, selection);
  const summary = buildSummary(filtered);
  const details = buildDetails(filtered);
  const damageByTarget = buildDamageByEntity(filtered, 'outgoing');
  const damageByWeapon = buildWeaponBreakdown(filtered, 'outgoing');
  const incomingBySource = buildDamageByEntity(filtered, 'incoming');
  const incomingByWeapon = buildWeaponBreakdown(filtered, 'incoming');
  const pilot = pilotEvents(filtered, filters.pilot);

  return {
    selection,
    hiddenSeries: initialHiddenSeries(filters),
    dpsOverTime: dpsOverTime(events),
    filteredEvents: filtered,
    summary,
    details,
    peakDps: peakDps(filtered),
    tankSupportRatio: tankSupportRatio(summary),
    neutPressureIn: neutPressureFor(summary, details.neutIn),
    neutPressureOut: neutPressureFor(summary, details.neutOut),
    qualitySegmentsOut: qualitySegments(details.qualityOut),
    qualitySegmentsIn: qualitySegments(details.qualityIn),
    damageByTarget,
    damageByWeapon,
    incomingBySource,
    incomingByWeapon,
    outgoingTableRows: pilotTableRows(damageByTarget, summary),
    incomingTableRows: pilotTableRows(incomingBySource, summary),
    logiTableRows: logiTableRows(filtered),
    damageByTargetItems: entityItems(damageByTarget),
    damageByWeaponItems: weaponItems(damageByWeapon),
    incomingBySourceItems: entityItems(incomingBySource),
    incomingByWeaponItems: weaponItems(incomingByWeapon),
    selectionLabel: selectionLabel(selection),
    engagement: engagementOf(pilot),
    engagementLog: engagementLog(pilot, new Set(ENGAGEMENT_TYPES), 'both'),
  };
}
