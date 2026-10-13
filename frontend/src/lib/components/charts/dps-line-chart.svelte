<script lang="ts">
  import { onMount } from 'svelte';
  import type { ChartData, ChartOptions, Plugin } from 'chart.js';

  import type { DateTimeRange, DpsDataPoint, SeriesKey } from '#lib/types.ts';

  import { Chart, TOOLTIP_STYLE } from './chart.ts';
  import { SERIES, indexAtPixel, selectionFromDrag } from './dps-series.ts';

  let {
    data,
    selection = null,
    hiddenSeries = new Set<SeriesKey>(),
    onSelection,
    onToggleSeries,
  }: {
    data: DpsDataPoint[];
    selection?: DateTimeRange | null;
    hiddenSeries?: Set<SeriesKey>;
    onSelection: (range: DateTimeRange | null) => void;
    onToggleSeries: (key: SeriesKey) => void;
  } = $props();

  const prefersReducedMotion =
    typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  let canvas: HTMLCanvasElement;
  let chart: Chart<'line'> | null = null;
  let dragStart: number | null = null;
  let dragEnd: number | null = null;
  let isDragging = false;

  function getIndexAtEvent(target: Chart<'line'>, clientX: number): number {
    const xScale = target.scales['x'];
    const rect = target.canvas.getBoundingClientRect();
    return indexAtPixel(data.length, (i) => xScale.getPixelForValue(i), clientX - rect.left);
  }

  function onMouseDown(event: MouseEvent) {
    if (!chart) {
      return;
    }

    const index = getIndexAtEvent(chart, event.clientX);

    dragStart = index;
    dragEnd = index;
    isDragging = true;
  }

  function onMouseMove(event: MouseEvent) {
    if (!isDragging || !chart) {
      return;
    }

    dragEnd = getIndexAtEvent(chart, event.clientX);
    chart.update('none');
  }

  function onMouseUp() {
    if (!isDragging) {
      return;
    }

    isDragging = false;

    if (dragStart !== null && dragEnd !== null) {
      onSelection(selectionFromDrag(data, dragStart, dragEnd));
    }
  }

  function clearSelection() {
    dragStart = null;
    dragEnd = null;
    onSelection(null);
  }

  const selectionPlugin: Plugin<'line'> = {
    id: 'rangeSelection',
    beforeDraw(target) {
      let startIdx: number | null = null;
      let endIdx: number | null = null;

      if (isDragging) {
        startIdx = Math.min(dragStart ?? 0, dragEnd ?? 0);
        endIdx = Math.max(dragStart ?? 0, dragEnd ?? 0);
      } else if (selection) {
        const start = selection.start;
        const end = selection.end;
        startIdx = data.findIndex((d) => d.datetime === start);
        endIdx = data.findIndex((d) => d.datetime === end);
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

      const { ctx } = target;
      const xScale = target.scales['x'];
      const yScale = target.scales['y'];

      const left = xScale.getPixelForValue(startIdx);
      const right = xScale.getPixelForValue(endIdx);

      ctx.save();
      ctx.fillStyle = 'rgba(251, 191, 36, 0.08)';
      ctx.fillRect(left, yScale.top, right - left, yScale.bottom - yScale.top);
      ctx.strokeStyle = 'rgba(251, 191, 36, 0.45)';
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 4]);
      ctx.strokeRect(left, yScale.top, right - left, yScale.bottom - yScale.top);
      ctx.restore();
    },
  };

  let chartData = $derived<ChartData<'line'>>({
    labels: data.map((d) => d.label),
    datasets: SERIES.map((s) => ({
      label: s.label,
      data: data.map(s.pick),
      borderColor: s.color,
      backgroundColor: s.bg,
      fill: false,
      tension: 0.3,
      pointRadius: 0,
      pointHitRadius: 10,
      borderWidth: 2,
      hidden: hiddenSeries.has(s.key),
    })),
  });

  const chartOptions: ChartOptions<'line'> = {
    responsive: true,
    maintainAspectRatio: false,
    animation: prefersReducedMotion ? false : { duration: 400 },
    interaction: {
      mode: 'index',
      intersect: false,
    },
    plugins: {
      legend: {
        position: 'top',
        labels: {
          usePointStyle: true,
          padding: 16,
          color: '#d4d4d8',
          font: { family: 'monospace', size: 12 },
        },
        onClick: (_e, legendItem) => {
          const idx = legendItem.datasetIndex;

          if (idx === undefined || idx < 0 || idx >= SERIES.length) {
            return;
          }

          onToggleSeries(SERIES[idx].key);
        },
      },
      tooltip: {
        ...TOOLTIP_STYLE,
        callbacks: {
          label: (context) => `${context.dataset.label}: ${Math.round(context.parsed.y ?? 0)} /s`,
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
  };

  onMount(() => {
    chart = new Chart(canvas, {
      type: 'line',
      data: chartData,
      options: chartOptions,
      plugins: [selectionPlugin],
    });
    return () => chart?.destroy();
  });

  $effect(() => {
    const next = chartData;
    if (chart) {
      chart.data = next;
      chart.update();
    }
  });

  // Redraw the selection band when the selection changes.
  $effect(() => {
    void selection;
    chart?.update('none');
  });
</script>

<div>
  <div class="mb-2 flex items-center justify-between">
    <p class="font-mono text-xs tracking-wider text-zinc-400 uppercase">
      Click and drag to select a time range
    </p>
    {#if selection}
      <button
        class="font-mono text-xs font-medium tracking-wider text-amber-300 uppercase hover:text-amber-200"
        onclick={clearSelection}
      >
        Clear selection
      </button>
    {/if}
  </div>
  <div
    class="h-80"
    role="presentation"
    data-testid="dps-chart"
    onmousedown={onMouseDown}
    onmousemove={onMouseMove}
    onmouseup={onMouseUp}
    onmouseleave={onMouseUp}
  >
    <canvas bind:this={canvas}></canvas>
  </div>
</div>
