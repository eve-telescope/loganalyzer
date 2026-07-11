<script setup lang="ts">
import { Head, Link, router } from '@inertiajs/vue3';
import { computed, onMounted, ref, watch } from 'vue';
import {
    create,
    download,
} from '@/actions/App/Http/Controllers/CombatLogController';

import DamageBarChart from '@/components/charts/DamageBarChart.vue';
import DamageDoughnutChart from '@/components/charts/DamageDoughnutChart.vue';
import DpsLineChart from '@/components/charts/DpsLineChart.vue';
import QualitySpectrum from '@/components/charts/QualitySpectrum.vue';
import AnimatedNumber from '@/components/ui/AnimatedNumber.vue';
import DataTable from '@/components/ui/DataTable.vue';

import type { Column } from '@/components/ui/DataTable.vue';
import EventTypeIcon from '@/components/ui/EventTypeIcon.vue';
import PilotAvatar from '@/components/ui/PilotAvatar.vue';
import ShipIcon from '@/components/ui/ShipIcon.vue';
import StatPanel from '@/components/ui/StatPanel.vue';
import StatRow from '@/components/ui/StatRow.vue';
import ZkillLink from '@/components/ui/ZkillLink.vue';
import {
    eveTimestampToIso,
    formatDateTime,
    formatTime,
    relativeTimeFromEve,
} from '@/lib/dates';
import type {
    CombatAnalysis,
    CombatEventData,
    DateTimeRange,
    DpsDataPoint,
    SeriesKey,
    TargetDamage,
    WeaponDamage,
} from '@/types';

const ALL_SERIES_KEYS: ReadonlyArray<SeriesKey> = [
    'dpsDealt',
    'dpsReceived',
    'logiReceived',
    'logiDealt',
    'neutIn',
    'neutOut',
];

const BUCKET_SECONDS = 5;

const props = defineProps<{
    analysis: CombatAnalysis;
    uuid?: string;
    rawLogAvailable?: boolean;
    pilotIds: Record<string, number>;
    shipTypeIds: Record<string, number>;
    filters: {
        from: string | null;
        to: string | null;
        pilot: string | null;
        hide: string[];
    };
}>();

const VALID_SERIES_KEYS = new Set<SeriesKey>(ALL_SERIES_KEYS);

const ogTitle = computed(() => `${props.analysis.listener} — Combat Analysis`);

const ogDescription = computed(() => {
    const s = summary.value;

    return `${formatNumber(s.totalDamageDealt)} damage dealt / ${formatNumber(s.totalDamageReceived)} received — ${formatNumber(Math.round(s.dpsDealt))} DPS avg — ${formatDuration(s.combatDurationSeconds)} combat`;
});

const selection = ref<DateTimeRange | null>(
    props.filters.from &&
        props.filters.to &&
        props.filters.from !== props.filters.to
        ? { start: props.filters.from, end: props.filters.to }
        : null,
);
const hiddenSeries = ref<Set<SeriesKey>>(
    new Set(
        props.filters.hide.filter((k): k is SeriesKey =>
            VALID_SERIES_KEYS.has(k as SeriesKey),
        ),
    ),
);

function toggleSeries(key: SeriesKey) {
    const next = new Set(hiddenSeries.value);

    if (next.has(key)) {
        next.delete(key);
    } else {
        next.add(key);
    }

    hiddenSeries.value = next;
}

const dpsOverTime = computed((): DpsDataPoint[] => {
    const events = props.analysis.events;

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
        const idx = Math.min(
            Math.floor((ms - firstMs) / 1000 / BUCKET_SECONDS),
            bucketCount - 1,
        );

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
});

const filteredEvents = computed((): CombatEventData[] => {
    if (!selection.value) {
        return props.analysis.events;
    }

    const { start, end } = selection.value;

    return props.analysis.events.filter((e) => {
        const dt = eveTimestampToIso(e.timestamp);

        return dt >= start && dt <= end;
    });
});

const summary = computed(() => buildSummary(filteredEvents.value));

/** EVE hit qualities from best roll to worst, misses last. */
const QUALITY_ORDER = [
    'Wrecks',
    'Smashes',
    'Penetrates',
    'Hits',
    'Glances Off',
    'Grazes',
    'Misses',
] as const;

const PEAK_WINDOW_SECONDS = 10;

interface BiggestHit {
    damage: number;
    weapon: string;
    pilot: string;
}

function buildDetails(events: CombatEventData[]) {
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

const details = computed(() => buildDetails(filteredEvents.value));

const peakDps = computed(() => {
    const events = filteredEvents.value;

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

        const idx = Math.floor(
            (parseTime(e.timestamp) - firstMs) / 1000 / PEAK_WINDOW_SECONDS,
        );
        const buckets =
            e.direction === 'outgoing' ? dealtBuckets : receivedBuckets;

        buckets.set(idx, (buckets.get(idx) ?? 0) + e.damage);
    }

    const peakOf = (buckets: Map<number, number>) =>
        buckets.size > 0
            ? Math.round(Math.max(...buckets.values()) / PEAK_WINDOW_SECONDS)
            : 0;

    return { dealt: peakOf(dealtBuckets), received: peakOf(receivedBuckets) };
});

/** HP repaired onto the listener per HP of damage taken. */
const tankSupportRatio = computed((): number | string => {
    if (summary.value.totalDamageReceived === 0) {
        return '—';
    }

    return (
        (summary.value.totalLogiReceived / summary.value.totalDamageReceived) *
        100
    );
});

function neutPressureFor(totalGj: number): number | string {
    const duration = summary.value.combatDurationSeconds;

    return duration === 0 || totalGj === 0 ? '—' : totalGj / duration;
}

const neutPressureIn = computed((): number | string =>
    neutPressureFor(details.value.neutIn),
);

const neutPressureOut = computed((): number | string =>
    neutPressureFor(details.value.neutOut),
);

const qualitySegmentsOut = computed(() =>
    QUALITY_ORDER.map((quality) => ({
        label: quality,
        count: details.value.qualityOut[quality] ?? 0,
    })),
);

const qualitySegmentsIn = computed(() =>
    QUALITY_ORDER.map((quality) => ({
        label: quality,
        count: details.value.qualityIn[quality] ?? 0,
    })),
);

const damageByTarget = computed((): Record<string, TargetDamage> =>
    buildDamageByEntity(filteredEvents.value, 'outgoing'),
);

const damageByWeapon = computed((): Record<string, WeaponDamage> =>
    buildWeaponBreakdown(filteredEvents.value, 'outgoing'),
);

const incomingBySource = computed((): Record<string, TargetDamage> =>
    buildDamageByEntity(filteredEvents.value, 'incoming'),
);

const incomingByWeapon = computed((): Record<string, WeaponDamage> =>
    buildWeaponBreakdown(filteredEvents.value, 'incoming'),
);

function buildSummary(events: CombatEventData[]) {
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
        dpsDealt:
            duration > 0
                ? Math.round((totalDamageDealt / duration) * 100) / 100
                : 0,
        dpsReceived:
            duration > 0
                ? Math.round((totalDamageReceived / duration) * 100) / 100
                : 0,
        totalOutgoingHits: outgoingHits,
        totalOutgoingMisses: outgoingMisses,
        totalIncomingHits: incomingHits,
        totalIncomingMisses: incomingMisses,
        totalLogiReceived: logiReceived,
        totalLogiDealt: logiDealt,
    };
}

function getDurationSeconds(events: CombatEventData[]): number {
    if (events.length < 2) {
        return 0;
    }

    const first = parseTime(events[0].timestamp);
    const last = parseTime(events[events.length - 1].timestamp);

    return Math.round((last - first) / 1000);
}

function parseTime(timestamp: string): number {
    const [date, time] = timestamp.split(' ');
    const [y, m, d] = date.split('.');
    const [h, min, s] = time.split(':');

    return new Date(+y, +m - 1, +d, +h, +min, +s).getTime();
}

function buildDamageByEntity(
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

    return Object.fromEntries(
        Object.entries(result).sort(([, a], [, b]) => b.damage - a.damage),
    );
}

function buildWeaponBreakdown(
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

    return Object.fromEntries(
        Object.entries(result).sort(([, a], [, b]) => b.damage - a.damage),
    );
}

// Table data
const outgoingTableRows = computed(() =>
    Object.entries(damageByTarget.value).map(([name, data]) => {
        const dur = summary.value.combatDurationSeconds;

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
    }),
);

const incomingTableRows = computed(() =>
    Object.entries(incomingBySource.value).map(([name, data]) => {
        const dur = summary.value.combatDurationSeconds;

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
    }),
);

const logiTableRows = computed(() => {
    const logiEvents = filteredEvents.value.filter(
        (e) => e.type === 'logistics',
    );
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
});

const pilotColumns: Column[] = [
    { key: 'name', label: 'Pilot' },
    { key: 'ship', label: 'Ship' },
    { key: 'damage', label: 'Damage', align: 'right' },
    { key: 'dps', label: 'DPS', align: 'right' },
    { key: 'hits', label: 'Hits', align: 'right' },
    { key: 'misses', label: 'Misses', align: 'right' },
    { key: 'hitRate', label: 'Hit %', align: 'right' },
];

const logiColumns: Column[] = [
    { key: 'name', label: 'Pilot' },
    { key: 'direction', label: 'Direction' },
    { key: 'ship', label: 'Ship' },
    { key: 'module', label: 'Module' },
    { key: 'amount', label: 'HP Repaired', align: 'right' },
    { key: 'count', label: 'Cycles', align: 'right' },
];

function formatDuration(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;

    return mins > 0 ? `${mins}m ${secs}s` : `${secs}s`;
}

function formatNumber(n: number): string {
    return n.toLocaleString();
}

const damageByTargetItems = computed(() =>
    Object.entries(damageByTarget.value)
        .sort(([, a], [, b]) => b.damage - a.damage)
        .map(([name, data]) => ({
            label: data.ship ? `${name} (${data.ship})` : name,
            value: data.damage,
        })),
);

const damageByWeaponItems = computed(() =>
    Object.entries(damageByWeapon.value)
        .sort(([, a], [, b]) => b.damage - a.damage)
        .map(([name, data]) => ({
            label: name,
            value: data.damage,
        })),
);

const incomingBySourceItems = computed(() =>
    Object.entries(incomingBySource.value)
        .sort(([, a], [, b]) => b.damage - a.damage)
        .map(([name, data]) => ({
            label: data.ship ? `${name} (${data.ship})` : name,
            value: data.damage,
        })),
);

const incomingByWeaponItems = computed(() =>
    Object.entries(incomingByWeapon.value)
        .sort(([, a], [, b]) => b.damage - a.damage)
        .map(([name, data]) => ({
            label: name,
            value: data.damage,
        })),
);

const selectionLabel = computed(() => {
    if (!selection.value) {
        return null;
    }

    const start = selection.value.start.split('T')[1] ?? selection.value.start;
    const end = selection.value.end.split('T')[1] ?? selection.value.end;

    return `${start} - ${end}`;
});

const activeTab = ref<'outgoing' | 'incoming' | 'logistics'>('outgoing');

function percentFormat(n: number): string {
    return `${n.toFixed(1)}%`;
}

function wholePercentFormat(n: number): string {
    return `${Math.round(n)}%`;
}

function gjFormat(n: number): string {
    return `${Math.round(n).toLocaleString()} GJ`;
}

function gjPerSecondFormat(n: number): string {
    return `${n.toFixed(1)} GJ/s`;
}

function accuracyValue(hits: number, misses: number): number | string {
    const total = hits + misses;

    return total > 0 ? (hits / total) * 100 : '—';
}

// Pilot engagement drilldown
const selectedPilot = ref<string | null>(props.filters.pilot);

function selectPilot(name: string) {
    selectedPilot.value = selectedPilot.value === name ? null : name;
}

const engagementDialog = ref<HTMLDialogElement | null>(null);

watch(
    selectedPilot,
    (pilot) => {
        const dialog = engagementDialog.value;

        if (!dialog) {
            return;
        }

        if (pilot && !dialog.open) {
            dialog.showModal();
        } else if (!pilot && dialog.open) {
            dialog.close();
        }
    },
    { flush: 'post' },
);

onMounted(() => {
    if (selectedPilot.value) {
        engagementDialog.value?.showModal();
    }
});

function onEngagementDialogClick(event: MouseEvent) {
    if (event.target === engagementDialog.value) {
        engagementDialog.value?.close();
    }
}

const ENGAGEMENT_TYPES = ['damage', 'logistics', 'neutralization'] as const;
type EngagementType = (typeof ENGAGEMENT_TYPES)[number];

const ENGAGEMENT_TYPE_LABELS: Record<EngagementType, string> = {
    damage: 'Damage',
    logistics: 'Logistics',
    neutralization: 'Energy',
};

const engagementTypeFilter = ref<Set<EngagementType>>(
    new Set(ENGAGEMENT_TYPES),
);
const engagementDirection = ref<'both' | 'outgoing' | 'incoming'>('both');

function toggleEngagementType(type: EngagementType) {
    const next = new Set(engagementTypeFilter.value);

    if (next.has(type)) {
        next.delete(type);
    } else {
        next.add(type);
    }

    engagementTypeFilter.value = next;
}

const pilotEvents = computed((): CombatEventData[] =>
    selectedPilot.value
        ? filteredEvents.value.filter(
              (e) => e.playerName === selectedPilot.value,
          )
        : [],
);

const engagement = computed(() => {
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

    for (const e of pilotEvents.value) {
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
});

const engagementLog = computed(() =>
    pilotEvents.value
        .filter((e) => engagementTypeFilter.value.has(e.type))
        .filter(
            (e) =>
                engagementDirection.value === 'both' ||
                e.direction === engagementDirection.value,
        )
        .map((e) => ({
            ...e,
            time: e.timestamp.split(' ')[1] ?? e.timestamp,
        })),
);

function eventColorClass(e: { type: string; direction: string }): string {
    if (e.type === 'damage') {
        return e.direction === 'outgoing' ? 'text-cyan-400' : 'text-red-400';
    }

    if (e.type === 'logistics') {
        return e.direction === 'incoming'
            ? 'text-emerald-400'
            : 'text-amber-400';
    }

    return e.direction === 'incoming' ? 'text-violet-400' : 'text-pink-400';
}

watch(
    [selection, hiddenSeries, selectedPilot],
    ([sel, hidden, pilot]) => {
        const query: Record<string, string> = {};

        if (sel) {
            query.from = sel.start;
            query.to = sel.end;
        }

        if (hidden.size > 0) {
            query.hide = ALL_SERIES_KEYS.filter((k) => hidden.has(k)).join(',');
        }

        if (pilot) {
            query.pilot = pilot;
        }

        const current = new URLSearchParams(window.location.search);

        if (
            current.get('from') === (query.from ?? null) &&
            current.get('to') === (query.to ?? null) &&
            current.get('hide') === (query.hide ?? null) &&
            current.get('pilot') === (query.pilot ?? null)
        ) {
            return;
        }

        router.get(window.location.pathname, query, {
            preserveState: true,
            preserveScroll: true,
            replace: true,
            only: ['filters'],
        });
    },
    { deep: true },
);
</script>

<template>
    <Head :title="ogTitle">
        <meta name="description" :content="ogDescription" />
        <meta property="og:type" content="website" />
        <meta property="og:site_name" content="Combat Log Analyzer" />
        <meta property="og:title" :content="ogTitle" />
        <meta property="og:description" :content="ogDescription" />
        <meta name="twitter:card" content="summary_large_image" />
        <meta name="twitter:title" :content="ogTitle" />
        <meta name="twitter:description" :content="ogDescription" />
    </Head>
    <div class="min-h-screen bg-zinc-950 text-zinc-300">
        <!-- Scanline overlay -->
        <div
            class="pointer-events-none fixed inset-0 opacity-[0.015]"
            style="
                background: repeating-linear-gradient(
                    0deg,
                    transparent,
                    transparent 2px,
                    rgba(161, 161, 170, 0.3) 2px,
                    rgba(161, 161, 170, 0.3) 4px
                );
            "
        />

        <div class="relative mx-auto max-w-7xl px-4 py-6 sm:px-6 lg:px-8">
            <!-- After-action report header -->
            <header class="mb-6 border-b border-zinc-800 pb-6">
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2">
                        <!-- Crosshair icon -->
                        <svg
                            class="h-4 w-4 text-amber-400"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                            stroke-width="1.5"
                        >
                            <circle cx="12" cy="12" r="10" />
                            <line x1="12" y1="2" x2="12" y2="6" />
                            <line x1="12" y1="18" x2="12" y2="22" />
                            <line x1="2" y1="12" x2="6" y2="12" />
                            <line x1="18" y1="12" x2="22" y2="12" />
                            <circle cx="12" cy="12" r="3" />
                        </svg>
                        <p
                            class="font-mono text-xs tracking-widest text-zinc-400 uppercase"
                        >
                            Combat Log Analyzer // After-Action Report
                        </p>
                    </div>
                    <div class="flex items-center gap-4">
                        <a
                            href="https://github.com/eve-telescope/loganalyzer"
                            target="_blank"
                            rel="noopener noreferrer"
                            title="View source on GitHub"
                            class="text-zinc-400 transition-colors hover:text-amber-300"
                        >
                            <svg
                                class="h-4 w-4"
                                fill="currentColor"
                                viewBox="0 0 24 24"
                            >
                                <path
                                    fill-rule="evenodd"
                                    clip-rule="evenodd"
                                    d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0 1 12 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0 0 22 12.017C22 6.484 17.522 2 12 2Z"
                                />
                            </svg>
                        </a>
                        <a
                            v-if="rawLogAvailable && uuid"
                            :href="download.url(uuid)"
                            class="flex items-center gap-1.5 font-mono text-xs tracking-wider text-zinc-400 uppercase transition-colors hover:text-amber-300"
                        >
                            <!-- Download icon -->
                            <svg
                                class="h-3.5 w-3.5"
                                fill="none"
                                stroke="currentColor"
                                viewBox="0 0 24 24"
                                stroke-width="1.5"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3"
                                />
                            </svg>
                            Raw Log
                        </a>
                        <Link
                            :href="create.url()"
                            class="flex items-center gap-1.5 font-mono text-xs tracking-wider text-zinc-400 uppercase transition-colors hover:text-amber-300"
                        >
                            <!-- Upload icon -->
                            <svg
                                class="h-3.5 w-3.5"
                                fill="none"
                                stroke="currentColor"
                                viewBox="0 0 24 24"
                                stroke-width="1.5"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5"
                                />
                            </svg>
                            New Log
                        </Link>
                    </div>
                </div>

                <h1
                    class="mt-5 font-mono text-2xl font-bold tracking-tight text-zinc-100 sm:text-3xl"
                >
                    {{ analysis.listener }}
                </h1>
                <p
                    class="mt-1.5 font-mono text-xs tracking-wider text-zinc-400 uppercase"
                >
                    {{ analysis.sessionStarted }}
                    <template
                        v-if="relativeTimeFromEve(analysis.sessionStarted)"
                    >
                        ({{ relativeTimeFromEve(analysis.sessionStarted) }})
                    </template>
                    //
                    {{ formatDuration(summary.combatDurationSeconds) }}
                    {{ selection ? 'selected' : '' }} //
                    <AnimatedNumber :value="filteredEvents.length" /> events //
                    <AnimatedNumber :value="details.pilotsInvolved" />
                    pilots
                </p>
            </header>

            <!-- Filter indicator -->
            <div
                v-if="selection"
                class="mb-4 flex items-center gap-2 border-l-2 border-amber-400 bg-amber-950/20 px-3 py-2 font-mono text-xs tracking-wider text-amber-300 uppercase"
            >
                <svg
                    class="h-3 w-3 shrink-0"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    stroke-width="2"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        d="M12 3v17.25m0 0c-1.472 0-2.882.265-4.185.75M12 20.25c1.472 0 2.882.265 4.185.75M18.75 4.97A48.416 48.416 0 0012 4.5c-2.291 0-4.545.16-6.75.47m13.5 0c1.01.143 2.01.317 3 .52m-3-.52l2.62 10.726c.122.499-.106 1.028-.589 1.202a5.988 5.988 0 01-2.031.352 5.988 5.988 0 01-2.031-.352c-.483-.174-.711-.703-.59-1.202L18.75 4.97zm-16.5.52c.99-.203 1.99-.377 3-.52m0 0l2.62 10.726c.122.499-.106 1.028-.589 1.202a5.989 5.989 0 01-2.031.352 5.989 5.989 0 01-2.031-.352c-.483-.174-.711-.703-.59-1.202L5.25 4.97z"
                    />
                </svg>
                Range: {{ selectionLabel }}
            </div>

            <!-- Stat panels -->
            <section class="mb-6 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
                <StatPanel title="Offense" accent="cyan">
                    <p
                        class="font-mono text-2xl font-bold text-cyan-400 tabular-nums"
                    >
                        <AnimatedNumber :value="summary.totalDamageDealt" />
                    </p>
                    <p class="mb-3 text-xs text-zinc-400">damage dealt</p>
                    <dl class="space-y-1.5">
                        <StatRow
                            label="DPS avg"
                            :value="Math.round(summary.dpsDealt)"
                        />
                        <StatRow
                            label="DPS peak (10s)"
                            :value="peakDps.dealt"
                        />
                        <StatRow
                            label="Accuracy"
                            :value="
                                accuracyValue(
                                    summary.totalOutgoingHits,
                                    summary.totalOutgoingMisses,
                                )
                            "
                            :format="percentFormat"
                        />
                        <StatRow
                            label="Biggest hit"
                            :value="details.biggestHitOut?.damage ?? '—'"
                            :hint="
                                details.biggestHitOut
                                    ? `${details.biggestHitOut.weapon} → ${details.biggestHitOut.pilot}`
                                    : undefined
                            "
                        />
                        <StatRow
                            label="Targets engaged"
                            :value="details.uniqueTargets"
                        />
                    </dl>
                </StatPanel>

                <StatPanel title="Defense" accent="red">
                    <p
                        class="font-mono text-2xl font-bold text-red-400 tabular-nums"
                    >
                        <AnimatedNumber :value="summary.totalDamageReceived" />
                    </p>
                    <p class="mb-3 text-xs text-zinc-400">damage received</p>
                    <dl class="space-y-1.5">
                        <StatRow
                            label="DPS avg"
                            :value="Math.round(summary.dpsReceived)"
                        />
                        <StatRow
                            label="DPS peak (10s)"
                            :value="peakDps.received"
                        />
                        <StatRow
                            label="Enemy accuracy"
                            :value="
                                accuracyValue(
                                    summary.totalIncomingHits,
                                    summary.totalIncomingMisses,
                                )
                            "
                            :format="percentFormat"
                        />
                        <StatRow
                            label="Hardest hit taken"
                            :value="details.biggestHitIn?.damage ?? '—'"
                            :hint="
                                details.biggestHitIn
                                    ? `${details.biggestHitIn.weapon} ← ${details.biggestHitIn.pilot}`
                                    : undefined
                            "
                        />
                        <StatRow
                            label="Unique attackers"
                            :value="details.uniqueAttackers"
                        />
                    </dl>
                </StatPanel>

                <StatPanel title="Logistics" accent="green">
                    <p
                        class="font-mono text-2xl font-bold text-emerald-400 tabular-nums"
                    >
                        <AnimatedNumber :value="summary.totalLogiReceived" />
                    </p>
                    <p class="mb-3 text-xs text-zinc-400">
                        hp repaired onto you
                    </p>
                    <dl class="space-y-1.5">
                        <StatRow
                            label="Rep cycles received"
                            :value="details.logiInCycles"
                        />
                        <StatRow
                            label="Tank support"
                            :value="tankSupportRatio"
                            :format="wholePercentFormat"
                            hint="HP repaired onto you per HP of damage taken"
                        />
                        <StatRow
                            label="HP repaired by you"
                            :value="summary.totalLogiDealt"
                        />
                        <StatRow
                            label="Rep cycles dealt"
                            :value="details.logiOutCycles"
                        />
                    </dl>
                </StatPanel>

                <StatPanel title="Energy Warfare" accent="purple">
                    <p
                        class="font-mono text-2xl font-bold text-purple-400 tabular-nums"
                    >
                        <AnimatedNumber :value="details.neutIn" />
                        <span class="text-sm font-medium text-purple-300/80">
                            GJ
                        </span>
                    </p>
                    <p class="mb-3 text-xs text-zinc-400">drained from you</p>
                    <dl class="space-y-1.5">
                        <StatRow
                            label="Pressure on you"
                            :value="neutPressureIn"
                            :format="gjPerSecondFormat"
                        />
                        <StatRow
                            label="Neut events in"
                            :value="details.neutInEvents"
                        />
                        <StatRow
                            label="Drained by you"
                            :value="details.neutOut"
                            :format="gjFormat"
                        />
                        <StatRow
                            label="Pressure by you"
                            :value="neutPressureOut"
                            :format="gjPerSecondFormat"
                        />
                        <StatRow
                            label="Neut events out"
                            :value="details.neutOutEvents"
                        />
                    </dl>
                </StatPanel>
            </section>

            <!-- Hit quality spectrums -->
            <section class="mb-6 grid gap-4 lg:grid-cols-2">
                <StatPanel title="Outgoing Hit Quality" accent="cyan">
                    <QualitySpectrum
                        :segments="qualitySegmentsOut"
                        hue="cyan"
                    />
                </StatPanel>
                <StatPanel title="Incoming Hit Quality" accent="red">
                    <QualitySpectrum :segments="qualitySegmentsIn" hue="red" />
                </StatPanel>
            </section>

            <!-- DPS Timeline -->
            <section class="mb-6">
                <div class="mb-3 flex items-center gap-2">
                    <!-- Chart icon -->
                    <svg
                        class="h-4 w-4 text-zinc-400"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                        stroke-width="1.5"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 013 19.875v-6.75zM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V8.625zM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V4.125z"
                        />
                    </svg>
                    <h2
                        class="font-mono text-sm font-medium tracking-widest text-zinc-300 uppercase"
                    >
                        Timeline
                    </h2>
                </div>
                <StatPanel>
                    <DpsLineChart
                        :data="dpsOverTime"
                        :selection="selection"
                        :hidden-series="hiddenSeries"
                        @update:selection="selection = $event"
                        @toggle-series="toggleSeries"
                    />
                </StatPanel>
            </section>

            <!-- Damage Breakdowns -->
            <section class="mb-6">
                <div class="mb-3 flex items-center gap-2">
                    <!-- Bolt icon -->
                    <svg
                        class="h-4 w-4 text-zinc-400"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                        stroke-width="1.5"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z"
                        />
                    </svg>
                    <h2
                        class="font-mono text-sm font-medium tracking-widest text-zinc-300 uppercase"
                    >
                        Damage Breakdown
                    </h2>
                </div>
                <div class="grid gap-4 lg:grid-cols-2">
                    <StatPanel title="Outgoing — By Target" accent="cyan">
                        <DamageBarChart :items="damageByTargetItems" />
                    </StatPanel>
                    <StatPanel title="Outgoing — By Weapon" accent="cyan">
                        <DamageDoughnutChart :items="damageByWeaponItems" />
                    </StatPanel>
                    <StatPanel title="Incoming — By Source" accent="red">
                        <DamageBarChart
                            :items="incomingBySourceItems"
                            color="#ef4444"
                        />
                    </StatPanel>
                    <StatPanel title="Incoming — By Weapon" accent="red">
                        <DamageDoughnutChart :items="incomingByWeaponItems" />
                    </StatPanel>
                </div>
            </section>

            <!-- Detailed Tables -->
            <section>
                <div class="mb-3 flex items-center gap-2">
                    <!-- Table icon -->
                    <svg
                        class="h-4 w-4 text-zinc-400"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                        stroke-width="1.5"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            d="M3.375 19.5h17.25m-17.25 0a1.125 1.125 0 01-1.125-1.125M3.375 19.5h7.5c.621 0 1.125-.504 1.125-1.125m-9.75 0V5.625m0 12.75v-1.5c0-.621.504-1.125 1.125-1.125m18.375 2.625V5.625m0 12.75c0 .621-.504 1.125-1.125 1.125m1.125-1.125v-1.5c0-.621-.504-1.125-1.125-1.125m0 3.75h-7.5A1.125 1.125 0 0112 18.375m9.75-12.75c0-.621-.504-1.125-1.125-1.125H3.375c-.621 0-1.125.504-1.125 1.125m19.5 0v1.5c0 .621-.504 1.125-1.125 1.125M2.25 5.625v1.5c0 .621.504 1.125 1.125 1.125m0 0h17.25m-17.25 0h7.5c.621 0 1.125.504 1.125 1.125M3.375 8.25c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125m17.25-3.75h-7.5c-.621 0-1.125.504-1.125 1.125m8.625-1.125c.621 0 1.125.504 1.125 1.125v1.5c0 .621-.504 1.125-1.125 1.125m-17.25 0h7.5m-7.5 0c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125M12 10.875v-1.5m0 1.5c0 .621-.504 1.125-1.125 1.125M12 10.875c0 .621.504 1.125 1.125 1.125m-2.25 0c.621 0 1.125.504 1.125 1.125M12 12h7.5m-7.5 0c-.621 0-1.125.504-1.125 1.125M21.375 12c.621 0 1.125.504 1.125 1.125v1.5c0 .621-.504 1.125-1.125 1.125M12 17.25v-5.25m0 5.25c0 .621.504 1.125 1.125 1.125h2.25c.621 0 1.125-.504 1.125-1.125v-1.5c0-.621-.504-1.125-1.125-1.125H12m0 3.75c0 .621-.504 1.125-1.125 1.125H8.625A1.125 1.125 0 017.5 17.25v-1.5c0-.621.504-1.125 1.125-1.125H12"
                        />
                    </svg>
                    <h2
                        class="font-mono text-sm font-medium tracking-widest text-zinc-300 uppercase"
                    >
                        Pilot Details
                    </h2>
                </div>
                <StatPanel>
                    <div class="mb-4 flex gap-px">
                        <button
                            v-for="tab in [
                                'outgoing',
                                'incoming',
                                'logistics',
                            ] as const"
                            :key="tab"
                            class="border-b-2 px-4 py-2 font-mono text-xs tracking-widest uppercase transition-colors"
                            :class="
                                activeTab === tab
                                    ? 'border-amber-400 text-amber-300'
                                    : 'border-transparent text-zinc-400 hover:text-zinc-200'
                            "
                            @click="activeTab = tab"
                        >
                            {{
                                tab === 'outgoing'
                                    ? 'Outgoing'
                                    : tab === 'incoming'
                                      ? 'Incoming'
                                      : 'Logistics'
                            }}
                        </button>
                    </div>

                    <DataTable
                        v-if="activeTab === 'outgoing'"
                        :columns="pilotColumns"
                        :rows="outgoingTableRows"
                        empty-text="No outgoing damage events"
                        clickable
                        selected-key="name"
                        :selected-value="selectedPilot"
                        @row-click="selectPilot(String($event.name))"
                    >
                        <template #cell-name="{ value }">
                            <span class="flex items-center gap-2">
                                <ShipIcon
                                    v-if="
                                        pilotIds[value] == null &&
                                        shipTypeIds[value] != null
                                    "
                                    :name="value"
                                    :type-id="shipTypeIds[value]"
                                />
                                <PilotAvatar
                                    v-else
                                    :name="value"
                                    :character-id="pilotIds[value] ?? null"
                                />
                                <span>{{ value }}</span>
                                <ZkillLink
                                    v-if="pilotIds[value] != null"
                                    :character-id="pilotIds[value]"
                                    :name="value"
                                />
                            </span>
                        </template>
                        <template #cell-ship="{ value }">
                            <span class="flex items-center gap-2">
                                <ShipIcon
                                    :name="value"
                                    :type-id="shipTypeIds[value] ?? null"
                                />
                                <span>{{ value }}</span>
                            </span>
                        </template>
                    </DataTable>
                    <DataTable
                        v-else-if="activeTab === 'incoming'"
                        :columns="pilotColumns"
                        :rows="incomingTableRows"
                        empty-text="No incoming damage events"
                        clickable
                        selected-key="name"
                        :selected-value="selectedPilot"
                        @row-click="selectPilot(String($event.name))"
                    >
                        <template #cell-name="{ value }">
                            <span class="flex items-center gap-2">
                                <ShipIcon
                                    v-if="
                                        pilotIds[value] == null &&
                                        shipTypeIds[value] != null
                                    "
                                    :name="value"
                                    :type-id="shipTypeIds[value]"
                                />
                                <PilotAvatar
                                    v-else
                                    :name="value"
                                    :character-id="pilotIds[value] ?? null"
                                />
                                <span>{{ value }}</span>
                                <ZkillLink
                                    v-if="pilotIds[value] != null"
                                    :character-id="pilotIds[value]"
                                    :name="value"
                                />
                            </span>
                        </template>
                        <template #cell-ship="{ value }">
                            <span class="flex items-center gap-2">
                                <ShipIcon
                                    :name="value"
                                    :type-id="shipTypeIds[value] ?? null"
                                />
                                <span>{{ value }}</span>
                            </span>
                        </template>
                    </DataTable>
                    <DataTable
                        v-else
                        :columns="logiColumns"
                        :rows="logiTableRows"
                        empty-text="No logistics events"
                        clickable
                        selected-key="name"
                        :selected-value="selectedPilot"
                        @row-click="selectPilot(String($event.name))"
                    >
                        <template #cell-name="{ value }">
                            <span class="flex items-center gap-2">
                                <ShipIcon
                                    v-if="
                                        pilotIds[value] == null &&
                                        shipTypeIds[value] != null
                                    "
                                    :name="value"
                                    :type-id="shipTypeIds[value]"
                                />
                                <PilotAvatar
                                    v-else
                                    :name="value"
                                    :character-id="pilotIds[value] ?? null"
                                />
                                <span>{{ value }}</span>
                                <ZkillLink
                                    v-if="pilotIds[value] != null"
                                    :character-id="pilotIds[value]"
                                    :name="value"
                                />
                            </span>
                        </template>
                        <template #cell-ship="{ value }">
                            <span class="flex items-center gap-2">
                                <ShipIcon
                                    :name="value"
                                    :type-id="shipTypeIds[value] ?? null"
                                />
                                <span>{{ value }}</span>
                            </span>
                        </template>
                    </DataTable>
                </StatPanel>
            </section>

            <!-- Pilot engagement drilldown -->
            <dialog
                ref="engagementDialog"
                class="m-auto w-full max-w-6xl bg-transparent p-4 backdrop:bg-zinc-950/60 backdrop:backdrop-blur-[2px]"
                @close="selectedPilot = null"
                @click="onEngagementDialogClick"
            >
                <div v-if="selectedPilot" class="h-[90vh] bg-zinc-950">
                    <StatPanel
                        :title="`Engagement — ${selectedPilot}`"
                        class="flex h-full flex-col overflow-hidden"
                    >
                        <div class="flex items-start justify-between gap-4">
                            <div class="flex items-center gap-3">
                                <ShipIcon
                                    v-if="
                                        pilotIds[selectedPilot] == null &&
                                        shipTypeIds[selectedPilot] != null
                                    "
                                    :name="selectedPilot"
                                    :type-id="shipTypeIds[selectedPilot]"
                                />
                                <PilotAvatar
                                    v-else
                                    :name="selectedPilot"
                                    :character-id="
                                        pilotIds[selectedPilot] ?? null
                                    "
                                    size="md"
                                />
                                <div>
                                    <p
                                        class="flex items-center gap-2 font-mono text-lg font-semibold text-zinc-100"
                                    >
                                        {{ selectedPilot }}
                                        <ZkillLink
                                            v-if="
                                                pilotIds[selectedPilot] != null
                                            "
                                            :character-id="
                                                pilotIds[selectedPilot]
                                            "
                                            :name="selectedPilot"
                                        />
                                    </p>
                                    <p
                                        v-if="engagement.ship"
                                        class="flex items-center gap-1.5 text-xs text-zinc-400"
                                    >
                                        <ShipIcon
                                            :name="engagement.ship"
                                            :type-id="
                                                shipTypeIds[engagement.ship] ??
                                                null
                                            "
                                        />
                                        {{ engagement.ship }}
                                    </p>
                                </div>
                            </div>
                            <button
                                class="font-mono text-xs tracking-wider text-zinc-400 uppercase transition-colors hover:text-amber-300"
                                @click="selectedPilot = null"
                            >
                                Close ✕
                            </button>
                        </div>

                        <div class="mt-4 grid gap-4 sm:grid-cols-2">
                            <div>
                                <p
                                    class="mb-2 font-mono text-xs tracking-widest text-cyan-400 uppercase"
                                >
                                    You → {{ selectedPilot }}
                                </p>
                                <dl class="space-y-1.5">
                                    <StatRow
                                        label="Damage dealt"
                                        :value="engagement.dealt"
                                    />
                                    <StatRow
                                        label="Hits / misses"
                                        :value="`${engagement.hitsOn} / ${engagement.missesOn}`"
                                    />
                                    <StatRow
                                        label="Accuracy"
                                        :value="
                                            accuracyValue(
                                                engagement.hitsOn,
                                                engagement.missesOn,
                                            )
                                        "
                                        :format="percentFormat"
                                    />
                                    <StatRow
                                        v-if="engagement.logiOut > 0"
                                        label="HP repaired onto them"
                                        :value="engagement.logiOut"
                                    />
                                    <StatRow
                                        v-if="engagement.neutOut > 0"
                                        label="Energy drained from them"
                                        :value="engagement.neutOut"
                                        :format="gjFormat"
                                    />
                                </dl>
                            </div>
                            <div>
                                <p
                                    class="mb-2 font-mono text-xs tracking-widest text-red-400 uppercase"
                                >
                                    {{ selectedPilot }} → You
                                </p>
                                <dl class="space-y-1.5">
                                    <StatRow
                                        label="Damage received"
                                        :value="engagement.received"
                                    />
                                    <StatRow
                                        label="Hits / misses"
                                        :value="`${engagement.hitsBy} / ${engagement.missesBy}`"
                                    />
                                    <StatRow
                                        label="Accuracy"
                                        :value="
                                            accuracyValue(
                                                engagement.hitsBy,
                                                engagement.missesBy,
                                            )
                                        "
                                        :format="percentFormat"
                                    />
                                    <StatRow
                                        v-if="engagement.logiIn > 0"
                                        label="HP repaired onto you"
                                        :value="engagement.logiIn"
                                    />
                                    <StatRow
                                        v-if="engagement.neutIn > 0"
                                        label="Energy drained from you"
                                        :value="engagement.neutIn"
                                        :format="gjFormat"
                                    />
                                </dl>
                            </div>
                        </div>

                        <div class="mt-5 flex flex-wrap items-center gap-2">
                            <button
                                v-for="direction in [
                                    'both',
                                    'outgoing',
                                    'incoming',
                                ] as const"
                                :key="direction"
                                class="border px-2.5 py-1 font-mono text-xs tracking-wider uppercase transition-colors"
                                :class="
                                    engagementDirection === direction
                                        ? 'border-amber-400/60 bg-amber-400/10 text-amber-300'
                                        : 'border-zinc-700 text-zinc-400 hover:text-zinc-200'
                                "
                                @click="engagementDirection = direction"
                            >
                                {{ direction }}
                            </button>
                            <span class="mx-1 h-4 w-px bg-zinc-700" />
                            <button
                                v-for="type in ENGAGEMENT_TYPES"
                                :key="type"
                                class="flex items-center gap-1.5 border px-2.5 py-1 font-mono text-xs tracking-wider uppercase transition-colors"
                                :class="
                                    engagementTypeFilter.has(type)
                                        ? 'border-amber-400/60 bg-amber-400/10 text-amber-300'
                                        : 'border-zinc-700 text-zinc-400 hover:text-zinc-200'
                                "
                                @click="toggleEngagementType(type)"
                            >
                                <EventTypeIcon :type="type" />
                                {{ ENGAGEMENT_TYPE_LABELS[type] }}
                            </button>
                        </div>

                        <ul
                            class="mt-3 min-h-0 flex-1 divide-y divide-zinc-800/60 overflow-y-auto"
                        >
                            <li
                                v-for="(e, i) in engagementLog"
                                :key="i"
                                class="flex items-center gap-3 py-1.5"
                            >
                                <span
                                    class="font-mono text-xs text-zinc-500 tabular-nums"
                                >
                                    {{ e.time }}
                                </span>
                                <span :class="eventColorClass(e)">
                                    <EventTypeIcon :type="e.type" />
                                </span>
                                <span
                                    class="font-mono text-xs"
                                    :class="eventColorClass(e)"
                                >
                                    {{ e.direction === 'outgoing' ? '→' : '←' }}
                                </span>
                                <span
                                    class="w-20 shrink-0 text-right font-mono text-sm font-medium tabular-nums"
                                    :class="eventColorClass(e)"
                                >
                                    {{ formatNumber(e.damage)
                                    }}{{
                                        e.type === 'neutralization' ? ' GJ' : ''
                                    }}
                                </span>
                                <span class="truncate text-xs text-zinc-300">
                                    {{ e.weapon }}
                                </span>
                                <span
                                    class="ml-auto shrink-0 font-mono text-[11px] text-zinc-500 uppercase"
                                >
                                    {{ e.quality }}
                                </span>
                            </li>
                            <li
                                v-if="engagementLog.length === 0"
                                class="py-4 text-center text-sm text-zinc-400"
                            >
                                No events match the filters
                            </li>
                        </ul>
                    </StatPanel>
                </div>
            </dialog>
        </div>
    </div>
</template>
