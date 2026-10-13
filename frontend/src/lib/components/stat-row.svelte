<script lang="ts">
  import AnimatedNumber from './animated-number.svelte';

  let {
    label,
    value,
    format,
    hint,
  }: {
    label: string;
    /** Numbers animate on change; strings render as-is (e.g. an em dash). */
    value: string | number;
    format?: (n: number) => string;
    hint?: string;
  } = $props();
</script>

<div class="flex items-baseline justify-between gap-3" title={hint}>
  <dt class="truncate text-xs text-zinc-400">{label}</dt>
  <dd class="shrink-0 font-mono text-sm font-medium text-zinc-100 tabular-nums">
    {#if typeof value === 'number'}
      <AnimatedNumber {value} {format} />
    {:else}{value}{/if}
  </dd>
</div>
