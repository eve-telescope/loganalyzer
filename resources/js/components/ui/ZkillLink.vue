<script setup lang="ts">
defineProps<{
    characterId: number;
    name: string;
}>();
</script>

<template>
    <a
        :href="`https://zkillboard.com/character/${characterId}/`"
        target="_blank"
        rel="noopener noreferrer"
        :title="`Open ${name} on zKillboard`"
        class="text-zinc-500 transition-colors hover:text-amber-300"
        @click.stop
    >
        <svg
            class="h-3.5 w-3.5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            stroke-width="1.5"
        >
            <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25"
            />
        </svg>
    </a>
</template>
