<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    title,
    accent,
    class: className = '',
    children,
  }: {
    title?: string;
    accent?: 'cyan' | 'red' | 'green' | 'orange' | 'purple' | 'default';
    class?: string;
    children?: Snippet;
  } = $props();

  const accentDotColor: Record<string, string> = {
    cyan: 'bg-cyan-400',
    red: 'bg-red-400',
    green: 'bg-emerald-400',
    orange: 'bg-amber-400',
    purple: 'bg-purple-400',
    default: 'bg-zinc-500',
  };
</script>

<div class="relative border border-zinc-700/60 bg-zinc-900/60 p-5 {className}">
  <span class="absolute top-0 left-0 h-2 w-2 border-t border-l border-zinc-500/60"></span>
  <span class="absolute top-0 right-0 h-2 w-2 border-t border-r border-zinc-500/60"></span>
  <span class="absolute bottom-0 left-0 h-2 w-2 border-b border-l border-zinc-500/60"></span>
  <span class="absolute right-0 bottom-0 h-2 w-2 border-r border-b border-zinc-500/60"></span>

  {#if title}
    <div class="mb-3 flex items-center gap-2">
      <span class="h-1.5 w-1.5 rounded-full {accentDotColor[accent ?? 'default']}"></span>
      <h3 class="font-mono text-xs tracking-widest text-zinc-400 uppercase">
        {title}
      </h3>
    </div>
  {/if}

  {@render children?.()}
</div>
