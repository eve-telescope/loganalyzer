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
                <tr class="border-b border-slate-700/50">
                    <th
                        v-for="col in columns"
                        :key="col.key"
                        class="px-3 py-2 font-mono text-[10px] font-medium tracking-wider text-slate-500 uppercase"
                        :class="col.align === 'right' ? 'text-right' : 'text-left'"
                    >
                        {{ col.label }}
                    </th>
                </tr>
            </thead>
            <tbody>
                <tr
                    v-for="(row, i) in rows"
                    :key="i"
                    class="border-b border-slate-800/50 transition-colors hover:bg-slate-800/30"
                >
                    <td
                        v-for="col in columns"
                        :key="col.key"
                        class="px-3 py-2 font-mono text-xs text-slate-300"
                        :class="col.align === 'right' ? 'text-right' : 'text-left'"
                    >
                        {{ getValue(row, col) }}
                    </td>
                </tr>
                <tr v-if="rows.length === 0">
                    <td
                        :colspan="columns.length"
                        class="px-3 py-6 text-center font-mono text-xs text-slate-600"
                    >
                        {{ emptyText ?? 'No data' }}
                    </td>
                </tr>
            </tbody>
        </table>
    </div>
</template>
