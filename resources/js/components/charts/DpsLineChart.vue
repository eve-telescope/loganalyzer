<script setup lang="ts">
import {
    Chart as ChartJS,
    CategoryScale,
    LinearScale,
    PointElement,
    LineElement,
    Title,
    Tooltip,
    Legend,
    Filler,
} from 'chart.js';
import type { Chart, Plugin } from 'chart.js';
import { computed, ref, shallowRef, watch } from 'vue';
import { Line } from 'vue-chartjs';

import type { DateTimeRange, DpsDataPoint, SeriesKey } from '@/types';

ChartJS.register(
    CategoryScale,
    LinearScale,
    PointElement,
    LineElement,
    Title,
    Tooltip,
    Legend,
    Filler,
);

const props = defineProps<{
    data: DpsDataPoint[];
    selection?: DateTimeRange | null;
    hiddenSeries?: Set<SeriesKey>;
}>();

const emit = defineEmits<{
    'update:selection': [range: DateTimeRange | null];
    'toggle-series': [key: SeriesKey];
}>();

const prefersReducedMotion =
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;

const chartRef = shallowRef<{ chart: Chart } | null>(null);
const dragStart = ref<number | null>(null);
const dragEnd = ref<number | null>(null);
const isDragging = ref(false);

watch(
    () => props.selection,
    () => {
        chartRef.value?.chart?.update('none');
    },
    { flush: 'post', immediate: true },
);

function getIndexAtEvent(chart: Chart, clientX: number): number {
    const xScale = chart.scales['x'];
    const rect = chart.canvas.getBoundingClientRect();
    const x = clientX - rect.left;
    const count = props.data.length;

    if (count === 0) {
        return 0;
    }

    let closest = 0;
    let closestDist = Infinity;

    for (let i = 0; i < count; i++) {
        const px = xScale.getPixelForValue(i);
        const dist = Math.abs(px - x);

        if (dist < closestDist) {
            closestDist = dist;
            closest = i;
        }
    }

    return closest;
}

function onMouseDown(event: MouseEvent) {
    const chart = chartRef.value?.chart;

    if (!chart) {
        return;
    }

    const index = getIndexAtEvent(chart, event.clientX);

    dragStart.value = index;
    dragEnd.value = index;
    isDragging.value = true;
}

function onMouseMove(event: MouseEvent) {
    if (!isDragging.value) {
        return;
    }

    const chart = chartRef.value?.chart;

    if (!chart) {
        return;
    }

    dragEnd.value = getIndexAtEvent(chart, event.clientX);
    chart.update('none');
}

function onMouseUp() {
    if (!isDragging.value) {
        return;
    }

    isDragging.value = false;

    if (dragStart.value !== null && dragEnd.value !== null) {
        const startIndex = Math.min(dragStart.value, dragEnd.value);
        const endIndex = Math.max(dragStart.value, dragEnd.value);

        if (
            startIndex === endIndex ||
            !props.data[startIndex] ||
            !props.data[endIndex]
        ) {
            emit('update:selection', null);
        } else {
            emit('update:selection', {
                start: props.data[startIndex].datetime,
                end: props.data[endIndex].datetime,
            });
        }
    }
}

function clearSelection() {
    dragStart.value = null;
    dragEnd.value = null;
    emit('update:selection', null);
}

const selectionPlugin: Plugin = {
    id: 'rangeSelection',
    beforeDraw(chart: Chart) {
        let startIdx: number | null = null;
        let endIdx: number | null = null;

        if (isDragging.value) {
            startIdx = Math.min(dragStart.value ?? 0, dragEnd.value ?? 0);
            endIdx = Math.max(dragStart.value ?? 0, dragEnd.value ?? 0);
        } else if (props.selection) {
            const start = props.selection.start;
            const end = props.selection.end;
            startIdx = props.data.findIndex((d) => d.datetime === start);
            endIdx = props.data.findIndex((d) => d.datetime === end);
        }

        if (
            startIdx === null ||
            endIdx === null ||
            startIdx < 0 ||
            endIdx < 0 ||
            startIdx === endIdx
        ) {
            return;
        }

        const { ctx } = chart;
        const xScale = chart.scales['x'];
        const yScale = chart.scales['y'];

        const left = xScale.getPixelForValue(startIdx);
        const right = xScale.getPixelForValue(endIdx);

        ctx.save();
        ctx.fillStyle = 'rgba(251, 191, 36, 0.08)';
        ctx.fillRect(
            left,
            yScale.top,
            right - left,
            yScale.bottom - yScale.top,
        );
        ctx.strokeStyle = 'rgba(251, 191, 36, 0.45)';
        ctx.lineWidth = 1;
        ctx.setLineDash([4, 4]);
        ctx.strokeRect(
            left,
            yScale.top,
            right - left,
            yScale.bottom - yScale.top,
        );
        ctx.restore();
    },
};

const SERIES: ReadonlyArray<{
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

const chartData = computed(() => {
    const hidden = props.hiddenSeries ?? new Set<SeriesKey>();

    return {
        labels: props.data.map((d) => d.label),
        datasets: SERIES.map((s) => ({
            label: s.label,
            data: props.data.map(s.pick),
            borderColor: s.color,
            backgroundColor: s.bg,
            fill: false,
            tension: 0.3,
            pointRadius: 0,
            pointHitRadius: 10,
            borderWidth: 2,
            hidden: hidden.has(s.key),
        })),
    };
});

const chartOptions = computed(() => ({
    responsive: true,
    maintainAspectRatio: false,
    animation: prefersReducedMotion ? (false as const) : { duration: 400 },
    interaction: {
        mode: 'index' as const,
        intersect: false,
    },
    plugins: {
        legend: {
            position: 'top' as const,
            labels: {
                usePointStyle: true,
                padding: 16,
                color: '#d4d4d8',
                font: { family: 'monospace', size: 12 },
            },
            onClick: (_e: unknown, legendItem: { datasetIndex?: number }) => {
                const idx = legendItem.datasetIndex;

                if (idx === undefined || idx < 0 || idx >= SERIES.length) {
                    return;
                }

                emit('toggle-series', SERIES[idx].key);
            },
        },
        tooltip: {
            backgroundColor: 'rgba(24, 24, 27, 0.95)',
            borderColor: 'rgba(63, 63, 70, 0.6)',
            borderWidth: 1,
            titleColor: '#f4f4f5',
            bodyColor: '#d4d4d8',
            titleFont: { family: 'monospace' },
            bodyFont: { family: 'monospace' },
            callbacks: {
                label: (context: {
                    dataset: { label?: string };
                    parsed: { y: number | null };
                }) =>
                    `${context.dataset.label}: ${Math.round(context.parsed.y ?? 0)} /s`,
            },
        },
    },
    scales: {
        x: {
            grid: { color: 'rgba(63, 63, 70, 0.35)' },
            ticks: {
                maxTicksLimit: 15,
                color: '#a1a1aa',
                font: { family: 'monospace', size: 11 },
            },
        },
        y: {
            beginAtZero: true,
            grid: { color: 'rgba(63, 63, 70, 0.35)' },
            ticks: {
                color: '#a1a1aa',
                font: { family: 'monospace', size: 11 },
            },
            title: {
                display: true,
                text: 'HP/s',
                color: '#a1a1aa',
                font: { family: 'monospace' },
            },
        },
    },
}));
</script>

<template>
    <div>
        <div class="mb-2 flex items-center justify-between">
            <p class="font-mono text-xs tracking-wider text-zinc-400 uppercase">
                Click and drag to select a time range
            </p>
            <button
                v-if="selection"
                class="font-mono text-xs font-medium tracking-wider text-amber-300 uppercase hover:text-amber-200"
                @click="clearSelection"
            >
                Clear selection
            </button>
        </div>
        <div
            class="h-80"
            @mousedown="onMouseDown"
            @mousemove="onMouseMove"
            @mouseup="onMouseUp"
            @mouseleave="onMouseUp"
        >
            <Line
                ref="chartRef"
                :data="chartData"
                :options="chartOptions"
                :plugins="[selectionPlugin]"
            />
        </div>
    </div>
</template>
