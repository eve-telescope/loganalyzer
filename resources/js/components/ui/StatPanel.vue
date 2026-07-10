<script setup lang="ts">
defineProps<{
    title?: string;
    accent?: 'cyan' | 'red' | 'green' | 'orange' | 'purple' | 'default';
}>();

const accentDotColor: Record<string, string> = {
    cyan: 'bg-cyan-400',
    red: 'bg-red-400',
    green: 'bg-emerald-400',
    orange: 'bg-amber-400',
    purple: 'bg-purple-400',
    default: 'bg-slate-500',
};
</script>

<template>
    <div class="relative border border-slate-700/50 bg-slate-900/60 p-5">
        <span
            class="absolute top-0 left-0 h-2 w-2 border-t border-l border-slate-500/50"
        />
        <span
            class="absolute top-0 right-0 h-2 w-2 border-t border-r border-slate-500/50"
        />
        <span
            class="absolute bottom-0 left-0 h-2 w-2 border-b border-l border-slate-500/50"
        />
        <span
            class="absolute right-0 bottom-0 h-2 w-2 border-r border-b border-slate-500/50"
        />

        <div v-if="title" class="mb-3 flex items-center gap-2">
            <span
                class="h-1.5 w-1.5 rounded-full"
                :class="accentDotColor[accent ?? 'default']"
            />
            <h3
                class="font-mono text-xs tracking-widest text-slate-400 uppercase"
            >
                {{ title }}
            </h3>
        </div>

        <slot />
    </div>
</template>
