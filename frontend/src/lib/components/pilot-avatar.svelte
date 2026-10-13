<script lang="ts">
  import { hueClass, initials, portraitUrl } from '#lib/avatar.ts';

  let {
    name,
    characterId = null,
    size = 'sm',
  }: { name: string; characterId?: number | null; size?: 'sm' | 'md' } = $props();

  let failedFor = $state<number | null>(null);

  let portrait = $derived(failedFor === characterId ? null : portraitUrl(characterId));
</script>

{#if portrait}
  <img
    src={portrait}
    alt="Portrait of {name}"
    loading="lazy"
    class="shrink-0 rounded-full object-cover {size === 'md' ? 'h-9 w-9' : 'h-6 w-6'}"
    onerror={() => (failedFor = characterId)}
  />
{:else}
  <span
    class="inline-flex shrink-0 items-center justify-center rounded-full font-mono font-semibold {hueClass(
      name,
    )} {size === 'md' ? 'h-9 w-9 text-xs' : 'h-6 w-6 text-[10px]'}"
    aria-hidden="true"
  >
    {initials(name)}
  </span>
{/if}
