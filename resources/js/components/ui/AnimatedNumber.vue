<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue';

const props = withDefaults(
    defineProps<{
        value: number;
        format?: (n: number) => string;
        duration?: number;
    }>(),
    { duration: 500 },
);

const display = ref(props.value);

let frame = 0;

const prefersReducedMotion =
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;

watch(
    () => props.value,
    (to, from) => {
        cancelAnimationFrame(frame);

        if (prefersReducedMotion) {
            display.value = to;

            return;
        }

        const start = performance.now();

        const step = (now: number) => {
            const progress = Math.min((now - start) / props.duration, 1);
            const eased = 1 - Math.pow(1 - progress, 3);

            display.value = from + (to - from) * eased;

            if (progress < 1) {
                frame = requestAnimationFrame(step);
            }
        };

        frame = requestAnimationFrame(step);
    },
);

onBeforeUnmount(() => cancelAnimationFrame(frame));

function defaultFormat(n: number): string {
    return Math.round(n).toLocaleString();
}
</script>

<template>
    <span>{{ (format ?? defaultFormat)(display) }}</span>
</template>
