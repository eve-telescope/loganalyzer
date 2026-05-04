<script setup lang="ts">
import { Head, Link, router } from '@inertiajs/vue3';
import { computed, ref, watch } from 'vue';
import { create } from '@/actions/App/Http/Controllers/CombatLogController';


import DamageBarChart from '@/components/charts/DamageBarChart.vue';
import DamageDoughnutChart from '@/components/charts/DamageDoughnutChart.vue';
import DpsLineChart from '@/components/charts/DpsLineChart.vue';
import DataTable from '@/components/ui/DataTable.vue';

import type { Column } from '@/components/ui/DataTable.vue';
import StatCard from '@/components/ui/StatCard.vue';
import { eveTimestampToIso, formatDateTime, formatTime } from '@/lib/dates';
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
    filters: {
        from: string | null;
        to: string | null;
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

const damageByTarget = computed(
    (): Record<string, TargetDamage> =>
        buildDamageByEntity(filteredEvents.value, 'outgoing'),
);

const damageByWeapon = computed(
    (): Record<string, WeaponDamage> =>
        buildWeaponBreakdown(filteredEvents.value, 'outgoing'),
);

const incomingBySource = computed(
    (): Record<string, TargetDamage> =>
        buildDamageByEntity(filteredEvents.value, 'incoming'),
);

const incomingByWeapon = computed(
    (): Record<string, WeaponDamage> =>
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

function hitRate(hits: number, misses: number): string {
    const total = hits + misses;

    if (total === 0) {
        return '0%';
    }

    return `${((hits / total) * 100).toFixed(1)}%`;
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

watch(
    [selection, hiddenSeries],
    ([sel, hidden]) => {
        const query: Record<string, string> = {};

        if (sel) {
            query.from = sel.start;
            query.to = sel.end;
        }

        if (hidden.size > 0) {
            query.hide = ALL_SERIES_KEYS.filter((k) => hidden.has(k)).join(',');
        }

        const current = new URLSearchParams(window.location.search);

        if (
            current.get('from') === (query.from ?? null) &&
            current.get('to') === (query.to ?? null) &&
            current.get('hide') === (query.hide ?? null)
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
        <meta name="twitter:card" content="summary" />
        <meta name="twitter:title" :content="ogTitle" />
        <meta name="twitter:description" :content="ogDescription" />
    </Head>
    <div class="min-h-screen bg-slate-950 text-slate-300">
        <!-- Scanline overlay -->
        <div
            class="pointer-events-none fixed inset-0 opacity-[0.015]"
            style="
                background: repeating-linear-gradient(
                    0deg,
                    transparent,
                    transparent 2px,
                    rgba(148, 163, 184, 0.3) 2px,
                    rgba(148, 163, 184, 0.3) 4px
                );
            "
        />

        <div class="relative mx-auto max-w-7xl px-4 py-6 sm:px-6 lg:px-8">
            <!-- Top bar -->
            <div
                class="mb-6 flex items-center justify-between border-b border-slate-800 pb-4"
            >
                <div class="flex items-center gap-3">
                    <!-- Crosshair icon -->
                    <svg
                        class="h-5 w-5 text-cyan-500"
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
                    <div>
                        <h1
                            class="font-mono text-sm font-bold tracking-widest text-slate-100 uppercase"
                        >
                            Combat Analysis
                        </h1>
                        <p
                            class="font-mono text-[10px] tracking-wider text-slate-600"
                        >
                            {{ analysis.listener }} //
                            {{ analysis.sessionStarted }}
                        </p>
                    </div>
                </div>
                <Link
                    :href="create.url()"
                    class="flex items-center gap-1.5 font-mono text-[10px] tracking-wider text-slate-500 uppercase transition-colors hover:text-cyan-400"
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

            <!-- Filter indicator -->
            <div
                v-if="selection"
                class="mb-4 flex items-center gap-2 border-l-2 border-cyan-500 bg-cyan-950/10 px-3 py-1.5 font-mono text-[10px] tracking-wider text-cyan-500 uppercase"
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

            <!-- Stats strip -->
            <div
                class="mb-6 flex flex-wrap items-start gap-6 border-b border-slate-800/50 pb-5 lg:gap-8"
            >
                <StatCard
                    label="DMG Dealt"
                    :value="formatNumber(summary.totalDamageDealt)"
                    :subtitle="`${hitRate(summary.totalOutgoingHits, summary.totalOutgoingMisses)} accuracy`"
                    accent="cyan"
                />
                <StatCard
                    label="DMG Received"
                    :value="formatNumber(summary.totalDamageReceived)"
                    :subtitle="`${formatNumber(summary.totalIncomingHits + summary.totalIncomingMisses)} events`"
                    accent="red"
                />
                <StatCard
                    label="DPS Out"
                    :value="formatNumber(Math.round(summary.dpsDealt))"
                    subtitle="avg/s"
                    accent="cyan"
                />
                <StatCard
                    label="DPS In"
                    :value="formatNumber(Math.round(summary.dpsReceived))"
                    subtitle="avg/s"
                    accent="red"
                />
                <StatCard
                    label="Logi In"
                    :value="formatNumber(summary.totalLogiReceived)"
                    subtitle="HP repaired"
                    accent="green"
                />
                <StatCard
                    v-if="summary.totalLogiDealt > 0"
                    label="Logi Out"
                    :value="formatNumber(summary.totalLogiDealt)"
                    subtitle="HP repaired"
                    accent="orange"
                />
                <StatCard
                    label="Duration"
                    :value="formatDuration(summary.combatDurationSeconds)"
                    :subtitle="selection ? 'selected' : 'total'"
                />
            </div>

            <!-- DPS Timeline -->
            <section class="mb-6">
                <div class="mb-3 flex items-center gap-2">
                    <!-- Chart icon -->
                    <svg
                        class="h-4 w-4 text-slate-600"
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
                        class="font-mono text-xs font-medium tracking-widest text-slate-500 uppercase"
                    >
                        Timeline
                    </h2>
                </div>
                <div
                    class="rounded border border-slate-800/60 bg-slate-900/40 p-5"
                >
                    <DpsLineChart
                        :data="dpsOverTime"
                        :selection="selection"
                        :hidden-series="hiddenSeries"
                        @update:selection="selection = $event"
                        @toggle-series="toggleSeries"
                    />
                </div>
            </section>

            <!-- Damage Breakdowns -->
            <section class="mb-6">
                <div class="mb-3 flex items-center gap-2">
                    <!-- Bolt icon -->
                    <svg
                        class="h-4 w-4 text-slate-600"
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
                        class="font-mono text-xs font-medium tracking-widest text-slate-500 uppercase"
                    >
                        Damage Breakdown
                    </h2>
                </div>
                <div class="grid gap-4 lg:grid-cols-2">
                    <div
                        class="rounded border border-slate-800/60 bg-slate-900/40 p-5"
                    >
                        <h3
                            class="mb-3 font-mono text-[10px] tracking-widest text-cyan-600 uppercase"
                        >
                            Outgoing &mdash; By Target
                        </h3>
                        <DamageBarChart :items="damageByTargetItems" />
                    </div>
                    <div
                        class="rounded border border-slate-800/60 bg-slate-900/40 p-5"
                    >
                        <h3
                            class="mb-3 font-mono text-[10px] tracking-widest text-cyan-600 uppercase"
                        >
                            Outgoing &mdash; By Weapon
                        </h3>
                        <DamageDoughnutChart :items="damageByWeaponItems" />
                    </div>
                    <div
                        class="rounded border border-slate-800/60 bg-slate-900/40 p-5"
                    >
                        <h3
                            class="mb-3 font-mono text-[10px] tracking-widest text-red-700 uppercase"
                        >
                            Incoming &mdash; By Source
                        </h3>
                        <DamageBarChart
                            :items="incomingBySourceItems"
                            color="rgb(239, 68, 68)"
                        />
                    </div>
                    <div
                        class="rounded border border-slate-800/60 bg-slate-900/40 p-5"
                    >
                        <h3
                            class="mb-3 font-mono text-[10px] tracking-widest text-red-700 uppercase"
                        >
                            Incoming &mdash; By Weapon
                        </h3>
                        <DamageDoughnutChart :items="incomingByWeaponItems" />
                    </div>
                </div>
            </section>

            <!-- Detailed Tables -->
            <section>
                <div class="mb-3 flex items-center gap-2">
                    <!-- Table icon -->
                    <svg
                        class="h-4 w-4 text-slate-600"
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
                        class="font-mono text-xs font-medium tracking-widest text-slate-500 uppercase"
                    >
                        Pilot Details
                    </h2>
                </div>
                <div
                    class="rounded border border-slate-800/60 bg-slate-900/40 p-5"
                >
                    <div class="mb-4 flex gap-px">
                        <button
                            v-for="tab in [
                                'outgoing',
                                'incoming',
                                'logistics',
                            ] as const"
                            :key="tab"
                            class="border-b-2 px-4 py-1.5 font-mono text-[10px] tracking-widest uppercase transition-colors"
                            :class="
                                activeTab === tab
                                    ? 'border-cyan-500 text-cyan-400'
                                    : 'border-transparent text-slate-600 hover:text-slate-400'
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
                    />
                    <DataTable
                        v-else-if="activeTab === 'incoming'"
                        :columns="pilotColumns"
                        :rows="incomingTableRows"
                        empty-text="No incoming damage events"
                    />
                    <DataTable
                        v-else
                        :columns="logiColumns"
                        :rows="logiTableRows"
                        empty-text="No logistics events"
                    />
                </div>
            </section>
        </div>
    </div>
</template>
