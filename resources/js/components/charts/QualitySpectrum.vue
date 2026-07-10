<script setup lang="ts">
import { computed } from 'vue';

export interface QualitySegment {
    label: string;
    count: number;
}

const props = defineProps<{
    segments: QualitySegment[];
    hue: 'cyan' | 'red';
}>();

/**
 * Hit quality is ordinal (Wrecks = best roll, Grazes = worst, Misses = none),
 * so each direction uses one hue running bright to dim, with misses in
 * neutral slate. Counts are always shown in the legend so color never
 * carries the information alone.
 */
const RAMPS: Record<string, string[]> = {
    cyan: [
        'bg-cyan-300',
        'bg-cyan-400',
        'bg-cyan-500',
        'bg-cyan-600',
        'bg-cyan-700',
        'bg-cyan-800',
        'bg-slate-700',
    ],
    red: [
        'bg-red-300',
        'bg-red-400',
        'bg-red-500',
        'bg-red-600',
        'bg-red-700',
        'bg-red-800',
        'bg-slate-700',
    ],
};

const total = computed(() =>
    props.segments.reduce((sum, s) => sum + s.count, 0),
);

const items = computed(() =>
    props.segments.map((segment, i) => ({
        ...segment,
        colorClass: RAMPS[props.hue][i] ?? 'bg-slate-700',
        share: total.value > 0 ? (segment.count / total.value) * 100 : 0,
    })),
);

const visibleItems = computed(() => items.value.filter((s) => s.count > 0));

function formatShare(share: number): string {
    return `${share.toFixed(1)}%`;
}
</script>

<template>
    <div v-if="total > 0">
        <div class="flex h-2 gap-0.5 overflow-hidden rounded-full">
            <div
                v-for="segment in visibleItems"
                :key="segment.label"
                :class="segment.colorClass"
                :style="{ width: `${segment.share}%` }"
                :title="`${segment.label}: ${segment.count} (${formatShare(segment.share)})`"
            />
        </div>
        <ul class="mt-2.5 flex flex-wrap gap-x-4 gap-y-1">
            <li
                v-for="segment in visibleItems"
                :key="segment.label"
                class="flex items-center gap-1.5"
            >
                <span
                    class="h-1.5 w-1.5 rounded-full"
                    :class="segment.colorClass"
                />
                <span class="text-[10px] text-slate-500">
                    {{ segment.label }}
                </span>
                <span class="font-mono text-[10px] text-slate-400 tabular-nums">
                    {{ segment.count }} · {{ formatShare(segment.share) }}
                </span>
            </li>
        </ul>
    </div>
    <p v-else class="text-[10px] text-slate-600">No attack events in range</p>
</template>
