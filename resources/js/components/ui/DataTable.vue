<script setup lang="ts">
export interface Column {
    key: string;
    label: string;
    align?: 'left' | 'right';
    format?: (value: unknown) => string;
}

defineProps<{
    columns: Column[];
    rows: Record<string, unknown>[];
    emptyText?: string;
    /** Rows become clickable and emit row-click. */
    clickable?: boolean;
    /** Key/value pair marking the currently selected row. */
    selectedKey?: string;
    selectedValue?: string | null;
}>();

defineEmits<{
    'row-click': [row: Record<string, unknown>];
}>();

function getValue(row: Record<string, unknown>, col: Column): string {
    const val = row[col.key];

    if (col.format) return col.format(val);
    if (typeof val === 'number') return val.toLocaleString();

    return String(val ?? '-');
}
</script>

<template>
    <div class="overflow-x-auto">
        <table class="w-full">
            <thead>
                <tr class="border-b border-zinc-700/60">
                    <th
                        v-for="col in columns"
                        :key="col.key"
                        class="px-3 py-2 font-mono text-xs font-medium tracking-wider text-zinc-400 uppercase"
                        :class="
                            col.align === 'right' ? 'text-right' : 'text-left'
                        "
                    >
                        {{ col.label }}
                    </th>
                </tr>
            </thead>
            <tbody>
                <tr
                    v-for="(row, i) in rows"
                    :key="i"
                    class="border-b border-zinc-800/60 transition-colors"
                    :class="[
                        clickable
                            ? 'cursor-pointer hover:bg-zinc-800/50'
                            : 'hover:bg-zinc-800/30',
                        selectedKey &&
                        selectedValue != null &&
                        row[selectedKey] === selectedValue
                            ? 'bg-zinc-800/60'
                            : '',
                    ]"
                    @click="clickable && $emit('row-click', row)"
                >
                    <td
                        v-for="col in columns"
                        :key="col.key"
                        class="px-3 py-2.5 font-mono text-sm text-zinc-200"
                        :class="
                            col.align === 'right' ? 'text-right' : 'text-left'
                        "
                    >
                        <slot
                            :name="`cell-${col.key}`"
                            :row="row"
                            :value="getValue(row, col)"
                        >
                            {{ getValue(row, col) }}
                        </slot>
                    </td>
                </tr>
                <tr v-if="rows.length === 0">
                    <td
                        :colspan="columns.length"
                        class="px-3 py-6 text-center font-mono text-sm text-zinc-400"
                    >
                        {{ emptyText ?? 'No data' }}
                    </td>
                </tr>
            </tbody>
        </table>
    </div>
</template>
