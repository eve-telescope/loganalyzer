<script lang="ts" module>
  export interface Column {
    key: string;
    label: string;
    align?: 'left' | 'right';
    format?: (value: unknown) => string;
  }

  export function cellValue(row: Record<string, unknown>, col: Column): string {
    const val = row[col.key];

    if (col.format) return col.format(val);
    if (typeof val === 'number') return val.toLocaleString();

    return String(val ?? '-');
  }
</script>

<script lang="ts" generics="Row extends Record<string, unknown>">
  import type { Snippet } from 'svelte';

  let {
    columns,
    rows,
    emptyText,
    clickable = false,
    selectedKey,
    selectedValue,
    onRowClick,
    cell,
  }: {
    columns: Column[];
    rows: Row[];
    emptyText?: string;
    /** Rows become clickable and call onRowClick. */
    clickable?: boolean;
    /** Key/value pair marking the currently selected row. */
    selectedKey?: string;
    selectedValue?: string | null;
    onRowClick?: (row: Row) => void;
    /** Renders a cell; without it the formatted value is shown. */
    cell?: Snippet<[{ key: string; value: string; row: Row }]>;
  } = $props();
</script>

<div class="overflow-x-auto">
  <table class="w-full">
    <thead>
      <tr class="border-b border-zinc-700/60">
        {#each columns as col (col.key)}
          <th
            class="px-3 py-2 font-mono text-xs font-medium tracking-wider text-zinc-400 uppercase {col.align ===
            'right'
              ? 'text-right'
              : 'text-left'}"
          >
            {col.label}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each rows as row, i (i)}
        <tr
          class="border-b border-zinc-800/60 transition-colors {clickable
            ? 'cursor-pointer hover:bg-zinc-800/50'
            : 'hover:bg-zinc-800/30'} {selectedKey &&
          selectedValue != null &&
          row[selectedKey] === selectedValue
            ? 'bg-zinc-800/60'
            : ''}"
          onclick={() => clickable && onRowClick?.(row)}
        >
          {#each columns as col (col.key)}
            <td
              class="px-3 py-2.5 font-mono text-sm text-zinc-200 {col.align === 'right'
                ? 'text-right'
                : 'text-left'}"
            >
              {#if cell}
                {@render cell({ key: col.key, value: cellValue(row, col), row })}
              {:else}
                {cellValue(row, col)}
              {/if}
            </td>
          {/each}
        </tr>
      {/each}
      {#if rows.length === 0}
        <tr>
          <td
            colspan={columns.length}
            class="px-3 py-6 text-center font-mono text-sm text-zinc-400"
          >
            {emptyText ?? 'No data'}
          </td>
        </tr>
      {/if}
    </tbody>
  </table>
</div>
