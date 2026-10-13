<script lang="ts">
  let { type }: { type: 'damage' | 'logistics' | 'neutralization' } = $props();
</script>

{#if type === 'damage'}
  <!-- Bolt: weapons fire -->
  <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.5">
    <path
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z"
    />
  </svg>
{:else if type === 'logistics'}
  <!-- Cross: remote repairs -->
  <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.5">
    <circle cx="12" cy="12" r="9" />
    <path stroke-linecap="round" d="M12 8v8M8 12h8" />
  </svg>
{:else}
  <!-- Drained battery: energy warfare -->
  <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.5">
    <rect x="3" y="8" width="15" height="8" rx="1.5" />
    <path stroke-linecap="round" d="M21 10.5v3M6.5 12h4" />
  </svg>
{/if}
