<script setup lang="ts">
defineProps<{
    type: 'damage' | 'logistics' | 'neutralization';
}>();
</script>

<template>
    <!-- Bolt: weapons fire -->
    <svg
        v-if="type === 'damage'"
        class="h-3.5 w-3.5"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        stroke-width="1.5"
    >
        <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z"
        />
    </svg>
    <!-- Cross: remote repairs -->
    <svg
        v-else-if="type === 'logistics'"
        class="h-3.5 w-3.5"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        stroke-width="1.5"
    >
        <circle cx="12" cy="12" r="9" />
        <path stroke-linecap="round" d="M12 8v8M8 12h8" />
    </svg>
    <!-- Drained battery: energy warfare -->
    <svg
        v-else
        class="h-3.5 w-3.5"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        stroke-width="1.5"
    >
        <rect x="3" y="8" width="15" height="8" rx="1.5" />
        <path stroke-linecap="round" d="M21 10.5v3M6.5 12h4" />
    </svg>
</template>
