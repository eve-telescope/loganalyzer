<script setup lang="ts">
defineProps<{
    label: string;
    value: string;
    hint?: string;
}>();
</script>

<template>
    <div
        class="flex items-baseline justify-between gap-3"
        :title="hint"
    >
        <dt class="truncate text-xs text-slate-400">{{ label }}</dt>
        <dd
            class="shrink-0 font-mono text-sm font-medium text-slate-100 tabular-nums"
        >
            {{ value }}
        </dd>
    </div>
</template>
