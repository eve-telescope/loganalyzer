<script lang="ts">
  import { onDestroy } from 'svelte';

  let {
    value,
    format,
    duration = 500,
  }: { value: number; format?: (n: number) => string; duration?: number } = $props();

  // svelte-ignore state_referenced_locally
  let display = $state(value);
  // svelte-ignore state_referenced_locally
  let previous = value;
  let frame = 0;

  const prefersReducedMotion =
    typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  $effect(() => {
    const to = value;
    const from = previous;
    previous = to;
    if (to === from) {
      return;
    }
    cancelAnimationFrame(frame);

    if (prefersReducedMotion) {
      display = to;
      return;
    }

    const start = performance.now();

    const step = (now: number) => {
      const progress = Math.min((now - start) / duration, 1);
      const eased = 1 - Math.pow(1 - progress, 3);

      display = from + (to - from) * eased;

      if (progress < 1) {
        frame = requestAnimationFrame(step);
      }
    };

    frame = requestAnimationFrame(step);
  });

  onDestroy(() => {
    if (typeof window !== 'undefined') {
      cancelAnimationFrame(frame);
    }
  });

  function defaultFormat(n: number): string {
    return Math.round(n).toLocaleString();
  }
</script>

<span>{(format ?? defaultFormat)(display)}</span>
