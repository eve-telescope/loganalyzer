<script setup lang="ts">
import AnimatedNumber from '@/components/ui/AnimatedNumber.vue';

defineProps<{
    label: string;
    /** Numbers animate on change; strings render as-is (e.g. an em dash). */
    value: string | number;
    format?: (n: number) => string;
    hint?: string;
}>();
</script>

<template>
    <div class="flex items-baseline justify-between gap-3" :title="hint">
        <dt class="truncate text-xs text-zinc-400">{{ label }}</dt>
        <dd
            class="shrink-0 font-mono text-sm font-medium text-zinc-100 tabular-nums"
        >
            <AnimatedNumber
                v-if="typeof value === 'number'"
                :value="value"
                :format="format"
            />
            <template v-else>{{ value }}</template>
        </dd>
    </div>
</template>
