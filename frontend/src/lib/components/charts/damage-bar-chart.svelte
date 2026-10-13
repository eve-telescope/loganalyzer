<script lang="ts">
  import { onMount } from 'svelte';
  import type { ChartData, ChartOptions } from 'chart.js';

  import { Chart, TOOLTIP_STYLE } from './chart.ts';

  let { items, color }: { items: { label: string; value: number }[]; color?: string } = $props();

  const DEFAULT_COLOR = '#0891b2';

  let canvas: HTMLCanvasElement;
  let chart: Chart<'bar'> | null = null;

  let chartData = $derived<ChartData<'bar'>>({
    labels: items.map((i) => i.label),
    datasets: [
      {
        data: items.map((i) => i.value),
        backgroundColor: items.map(() => color ?? DEFAULT_COLOR),
        borderRadius: 4,
        borderSkipped: false,
      },
    ],
  });

  const chartOptions: ChartOptions<'bar'> = {
    responsive: true,
    maintainAspectRatio: false,
    indexAxis: 'y',
    plugins: {
      legend: { display: false },
      tooltip: {
        ...TOOLTIP_STYLE,
        callbacks: {
          label: (context) => `${(context.parsed.x ?? 0).toLocaleString()} damage`,
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

  onMount(() => {
    chart = new Chart(canvas, { type: 'bar', data: chartData, options: chartOptions });
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
