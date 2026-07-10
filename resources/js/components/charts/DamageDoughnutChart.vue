<script setup lang="ts">
import { Chart as ChartJS, ArcElement, Title, Tooltip, Legend } from 'chart.js';
import { computed } from 'vue';
import { Doughnut } from 'vue-chartjs';

ChartJS.register(ArcElement, Title, Tooltip, Legend);

const props = defineProps<{
    items: { label: string; value: number }[];
}>();

// CVD-validated categorical order; assigned by position, never cycled per hue.
const colors = [
    '#0891b2',
    '#ef4444',
    '#059669',
    '#d97706',
    '#8b5cf6',
    '#db2777',
    '#0284c7',
    '#64748b',
];

const chartData = computed(() => ({
    labels: props.items.map((i) => i.label),
    datasets: [
        {
            data: props.items.map((i) => i.value),
            backgroundColor: props.items.map(
                (_, idx) => colors[idx % colors.length],
            ),
            borderWidth: 2,
            borderColor: '#020617',
        },
    ],
}));

const chartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    cutout: '60%',
    plugins: {
        legend: {
            position: 'bottom' as const,
            labels: {
                usePointStyle: true,
                padding: 12,
                color: '#cbd5e1',
                font: { family: 'monospace', size: 12 },
            },
        },
        tooltip: {
            backgroundColor: 'rgba(15, 23, 42, 0.95)',
            borderColor: 'rgba(51, 65, 85, 0.5)',
            borderWidth: 1,
            titleColor: '#e2e8f0',
            bodyColor: '#cbd5e1',
            titleFont: { family: 'monospace' },
            bodyFont: { family: 'monospace' },
            callbacks: {
                label: (context: {
                    label: string;
                    parsed: number;
                    dataset: { data: number[] };
                }) => {
                    const total = context.dataset.data.reduce(
                        (a: number, b: number) => a + b,
                        0,
                    );
                    const percentage = ((context.parsed / total) * 100).toFixed(
                        1,
                    );

                    return `${context.label}: ${context.parsed.toLocaleString()} (${percentage}%)`;
                },
            },
        },
    },
};
</script>

<template>
    <div class="h-80">
        <Doughnut :data="chartData" :options="chartOptions" />
    </div>
</template>
