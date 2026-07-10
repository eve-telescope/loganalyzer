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
        <dt class="truncate text-[10px] text-slate-600">{{ label }}</dt>
        <dd
            class="shrink-0 font-mono text-xs font-medium text-slate-300 tabular-nums"
        >
            {{ value }}
        </dd>
    </div>
</template>
