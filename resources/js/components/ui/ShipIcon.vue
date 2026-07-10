<script setup lang="ts">
import { computed, ref, watch } from 'vue';

const props = withDefaults(
    defineProps<{
        name: string;
        typeId?: number | null;
    }>(),
    { typeId: null },
);

const iconFailed = ref(false);

watch(
    () => props.typeId,
    () => {
        iconFailed.value = false;
    },
);

const iconUrl = computed(() =>
    props.typeId && !iconFailed.value
        ? `https://images.evetech.net/types/${props.typeId}/icon?size=32`
        : null,
);
</script>

<template>
    <img
        v-if="iconUrl"
        :src="iconUrl"
        :alt="`Icon of ${name}`"
        loading="lazy"
        class="h-5 w-5 shrink-0 rounded-sm object-cover"
        @error="iconFailed = true"
    />
</template>
