<script setup lang="ts">
defineProps<{
    label: string;
    value: string | number;
    subtitle?: string;
    accent?: 'cyan' | 'red' | 'green' | 'orange' | 'default';
}>();

const accentValueColor: Record<string, string> = {
    cyan: 'text-cyan-400',
    red: 'text-red-400',
    green: 'text-emerald-400',
    orange: 'text-amber-400',
    default: 'text-slate-200',
};

const accentBarColor: Record<string, string> = {
    cyan: 'bg-cyan-400',
    red: 'bg-red-400',
    green: 'bg-emerald-400',
    orange: 'bg-amber-400',
    default: 'bg-slate-600',
};
</script>

<template>
    <div class="group flex items-center gap-3">
        <div
            class="h-8 w-0.5 rounded-full"
            :class="accentBarColor[accent ?? 'default']"
        />
        <div class="min-w-0">
            <p class="truncate font-mono text-[10px] tracking-wider text-slate-500 uppercase">
                {{ label }}
            </p>
            <p
                class="font-mono text-lg font-bold leading-tight"
                :class="accentValueColor[accent ?? 'default']"
            >
                {{ value }}
            </p>
            <p
                v-if="subtitle"
                class="truncate text-[10px] text-slate-600"
            >
                {{ subtitle }}
            </p>
        </div>
    </div>
</template>
