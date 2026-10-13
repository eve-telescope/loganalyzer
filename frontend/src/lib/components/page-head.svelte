<script lang="ts">
  import type { HeadData } from '#lib/seo.ts';

  let { head }: { head: HeadData } = $props();
</script>

<svelte:head>
  <title>{head.title}</title>
  {#each head.meta as tag (tag.key)}
    {#if tag.attribute === 'name'}
      <meta name={tag.key} content={tag.content} />
    {:else}
      <meta property={tag.key} content={tag.content} />
    {/if}
  {/each}
</svelte:head>
