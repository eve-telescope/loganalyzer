<script setup lang="ts">
import { computed, ref, watch } from 'vue';

const props = withDefaults(
    defineProps<{
        name: string;
        characterId?: number | null;
        size?: 'sm' | 'md';
    }>(),
    { characterId: null, size: 'sm' },
);

const portraitFailed = ref(false);

watch(
    () => props.characterId,
    () => {
        portraitFailed.value = false;
    },
);

const portraitUrl = computed(() =>
    props.characterId && !portraitFailed.value
        ? `https://images.evetech.net/characters/${props.characterId}/portrait?size=64`
        : null,
);

/** Fallback for NPCs and unresolved names: deterministic initials avatar. */
const HUES = [
    'bg-zinc-700 text-zinc-200',
    'bg-stone-700 text-stone-200',
    'bg-zinc-600 text-zinc-100',
    'bg-neutral-700 text-neutral-200',
    'bg-stone-600 text-stone-100',
];

const initials = computed(() =>
    props.name
        .split(/\s+/)
        .filter(Boolean)
        .slice(0, 2)
        .map((part) => part[0]!.toUpperCase())
        .join(''),
);

const hueClass = computed(() => {
    let hash = 0;

    for (const char of props.name) {
        hash = (hash * 31 + char.charCodeAt(0)) | 0;
    }

    return HUES[Math.abs(hash) % HUES.length];
});
</script>

<template>
    <img
        v-if="portraitUrl"
        :src="portraitUrl"
        :alt="`Portrait of ${name}`"
        loading="lazy"
        class="shrink-0 rounded-full object-cover"
        :class="size === 'md' ? 'h-9 w-9' : 'h-6 w-6'"
        @error="portraitFailed = true"
    />
    <span
        v-else
        class="inline-flex shrink-0 items-center justify-center rounded-full font-mono font-semibold"
        :class="[
            hueClass,
            size === 'md' ? 'h-9 w-9 text-xs' : 'h-6 w-6 text-[10px]',
        ]"
        aria-hidden="true"
    >
        {{ initials }}
    </span>
</template>
