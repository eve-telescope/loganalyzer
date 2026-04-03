<script setup lang="ts">
import {
    Chart as ChartJS,
    CategoryScale,
    LinearScale,
    BarElement,
    Title,
    Tooltip,
    Legend,
} from 'chart.js';
import { computed } from 'vue';
import { Bar } from 'vue-chartjs';

ChartJS.register(
    CategoryScale,
    LinearScale,
    BarElement,
    Title,
    Tooltip,
    Legend,
);

const props = defineProps<{
    items: { label: string; value: number }[];
    color?: string;
}>();

const colors = [
    'rgb(34, 211, 238)',
    'rgb(52, 211, 153)',
    'rgb(251, 191, 36)',
    'rgb(239, 68, 68)',
    'rgb(168, 85, 247)',
    'rgb(236, 72, 153)',
    'rgb(56, 189, 248)',
    'rgb(148, 163, 184)',
];

const chartData = computed(() => ({
    labels: props.items.map((i) => i.label),
    datasets: [
        {
            data: props.items.map((i) => i.value),
            backgroundColor: props.color
                ? props.items.map(() => props.color!)
                : props.items.map((_, idx) => colors[idx % colors.length]),
            borderRadius: 2,
            borderSkipped: false,
        },
    ],
}));

const chartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    indexAxis: 'y' as const,
    plugins: {
        legend: { display: false },
        tooltip: {
            backgroundColor: 'rgba(15, 23, 42, 0.95)',
            borderColor: 'rgba(51, 65, 85, 0.5)',
            borderWidth: 1,
            titleColor: '#e2e8f0',
            bodyColor: '#94a3b8',
            titleFont: { family: 'monospace' },
            bodyFont: { family: 'monospace' },
            callbacks: {
                label: (context: { parsed: { x: number | null } }) =>
                    `${(context.parsed.x ?? 0).toLocaleString()} damage`,
            },
        },
    },
    scales: {
        x: {
            beginAtZero: true,
            grid: { color: 'rgba(51, 65, 85, 0.3)' },
            ticks: {
                color: '#475569',
                font: { family: 'monospace', size: 10 },
            },
        },
        y: {
            grid: { display: false },
            ticks: {
                color: '#94a3b8',
                font: { family: 'monospace', size: 10 },
            },
        },
    },
};
</script>

<template>
    <div class="h-80">
        <Bar :data="chartData" :options="chartOptions" />
    </div>
</template>
