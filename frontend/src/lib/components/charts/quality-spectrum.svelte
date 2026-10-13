<script lang="ts">
  import {
    formatShare,
    spectrumTotal,
    visibleSpectrumItems,
    type QualitySegment,
  } from '#lib/spectrum.ts';

  let { segments, hue }: { segments: QualitySegment[]; hue: 'cyan' | 'red' } = $props();

  let total = $derived(spectrumTotal(segments));
  let visibleItems = $derived(visibleSpectrumItems(segments, hue));
</script>

{#if total > 0}
  <div>
    <div class="flex h-2.5 gap-0.5 overflow-hidden rounded-full">
      {#each visibleItems as segment (segment.label)}
        <div
          class="transition-all duration-500 motion-reduce:transition-none {segment.colorClass}"
          style:width="{segment.share}%"
          title="{segment.label}: {segment.count} ({formatShare(segment.share)})"
        ></div>
      {/each}
    </div>
    <ul class="mt-2.5 flex flex-wrap gap-x-4 gap-y-1">
      {#each visibleItems as segment (segment.label)}
        <li class="flex items-center gap-1.5">
          <span class="h-1.5 w-1.5 rounded-full {segment.colorClass}"></span>
          <span class="text-xs text-zinc-300">
            {segment.label}
          </span>
          <span class="font-mono text-xs text-zinc-200 tabular-nums">
            {segment.count} · {formatShare(segment.share)}
          </span>
        </li>
      {/each}
    </ul>
  </div>
{:else}
  <p class="text-xs text-zinc-400">No attack events in range</p>
{/if}
