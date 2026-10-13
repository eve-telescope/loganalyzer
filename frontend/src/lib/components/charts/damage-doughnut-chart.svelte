<script lang="ts">
  import { onMount } from 'svelte';
  import type { ChartData, ChartOptions } from 'chart.js';

  import { Chart, TOOLTIP_STYLE } from './chart.ts';

  let { items }: { items: { label: string; value: number }[] } = $props();

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

  let canvas: HTMLCanvasElement;
  let chart: Chart<'doughnut'> | null = null;

  let chartData = $derived<ChartData<'doughnut'>>({
    labels: items.map((i) => i.label),
    datasets: [
      {
        data: items.map((i) => i.value),
        backgroundColor: items.map((_, idx) => colors[idx % colors.length]),
        borderWidth: 2,
        borderColor: '#09090b',
      },
    ],
  });

  const chartOptions: ChartOptions<'doughnut'> = {
    responsive: true,
    maintainAspectRatio: false,
    cutout: '60%',
    plugins: {
      legend: {
        position: 'bottom',
        labels: {
          usePointStyle: true,
          padding: 12,
          color: '#d4d4d8',
          font: { family: 'monospace', size: 12 },
        },
      },
      tooltip: {
        ...TOOLTIP_STYLE,
        callbacks: {
          label: (context) => {
            const total = (context.dataset.data as number[]).reduce((a, b) => a + b, 0);
            const percentage = ((context.parsed / total) * 100).toFixed(1);

            return `${context.label}: ${context.parsed.toLocaleString()} (${percentage}%)`;
          },
        },
      },
    },
  };

  onMount(() => {
    chart = new Chart(canvas, {
      type: 'doughnut',
      data: chartData,
      options: chartOptions,
    });
    return () => chart?.destroy();
  });

  $effect(() => {
    const data = chartData;
    if (chart) {
      chart.data = data;
      chart.update();
    }
  });
</script>

<div class="h-80">
  <canvas bind:this={canvas}></canvas>
</div>
