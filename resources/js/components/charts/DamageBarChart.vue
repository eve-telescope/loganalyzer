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

const DEFAULT_COLOR = '#0891b2';

const chartData = computed(() => ({
    labels: props.items.map((i) => i.label),
    datasets: [
        {
            data: props.items.map((i) => i.value),
            backgroundColor: props.items.map(
                () => props.color ?? DEFAULT_COLOR,
            ),
            borderRadius: 4,
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
            backgroundColor: 'rgba(24, 24, 27, 0.95)',
            borderColor: 'rgba(63, 63, 70, 0.6)',
            borderWidth: 1,
            titleColor: '#f4f4f5',
            bodyColor: '#d4d4d8',
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
            grid: { color: 'rgba(63, 63, 70, 0.35)' },
            ticks: {
                color: '#a1a1aa',
                font: { family: 'monospace', size: 11 },
            },
        },
        y: {
            grid: { display: false },
            ticks: {
                color: '#d4d4d8',
                font: { family: 'monospace', size: 11 },
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
