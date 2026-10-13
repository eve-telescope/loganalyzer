<script lang="ts">
  import { shipIconUrl } from '#lib/avatar.ts';

  let { name, typeId = null }: { name: string; typeId?: number | null } = $props();

  let failedFor = $state<number | null>(null);

  let icon = $derived(failedFor === typeId ? null : shipIconUrl(typeId));
</script>

{#if icon}
  <img
    src={icon}
    alt="Icon of {name}"
    loading="lazy"
    class="h-5 w-5 shrink-0 rounded-sm object-cover"
    onerror={() => (failedFor = typeId)}
  />
{/if}
