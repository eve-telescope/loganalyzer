<script setup lang="ts">
import { Chart as ChartJS, ArcElement, Title, Tooltip, Legend } from 'chart.js';
import { computed } from 'vue';
import { Doughnut } from 'vue-chartjs';

ChartJS.register(ArcElement, Title, Tooltip, Legend);

const props = defineProps<{
    items: { label: string; value: number }[];
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
            backgroundColor: props.items.map(
                (_, idx) => colors[idx % colors.length],
            ),
            borderWidth: 1,
            borderColor: 'rgb(15, 23, 42)',
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
                color: '#94a3b8',
                font: { family: 'monospace', size: 10 },
            },
        },
        tooltip: {
            backgroundColor: 'rgba(15, 23, 42, 0.95)',
            borderColor: 'rgba(51, 65, 85, 0.5)',
            borderWidth: 1,
            titleColor: '#e2e8f0',
            bodyColor: '#94a3b8',
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
