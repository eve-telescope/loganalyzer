<script setup lang="ts">
import { ref } from 'vue';

const emit = defineEmits<{
    file: [file: File];
    error: [message: string];
}>();

const props = defineProps<{
    accept?: string;
    error?: string;
    maxSizeMb: number;
}>();

const isDragging = ref(false);
const fileName = ref<string | null>(null);

function processFile(file: File) {
    const maxBytes = props.maxSizeMb * 1024 * 1024;

    if (file.size > maxBytes) {
        fileName.value = null;
        emit('error', `File is too large. Maximum size is ${props.maxSizeMb}MB.`);
        return;
    }

    fileName.value = file.name;
    emit('file', file);
}

function handleDrop(event: DragEvent) {
    isDragging.value = false;
    const file = event.dataTransfer?.files[0];

    if (file) {
        processFile(file);
    }
}

function handleFileInput(event: Event) {
    const target = event.target as HTMLInputElement;
    const file = target.files?.[0];

    if (file) {
        processFile(file);
    }
}
</script>

<template>
    <div
        class="relative rounded border-2 border-dashed transition-all duration-200"
        :class="[
            isDragging
                ? 'border-cyan-400 bg-cyan-950/20 shadow-[0_0_20px_rgba(34,211,238,0.1)]'
                : error
                  ? 'border-red-500/50 bg-red-950/10'
                  : 'border-slate-600 bg-slate-800/40 hover:border-slate-500 hover:bg-slate-800/60',
        ]"
        @dragover.prevent="isDragging = true"
        @dragleave.prevent="isDragging = false"
        @drop.prevent="handleDrop"
    >
        <label class="flex cursor-pointer flex-col items-center gap-3 p-10">
            <svg
                class="h-10 w-10"
                :class="isDragging ? 'text-cyan-400' : 'text-slate-500'"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
            >
                <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="1.5"
                    d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
                />
            </svg>
            <div class="text-center">
                <p
                    v-if="fileName"
                    class="font-mono text-sm font-medium text-cyan-400"
                >
                    {{ fileName }}
                </p>
                <template v-else>
                    <p class="text-sm font-medium text-slate-300">
                        Drop your combat log here or
                        <span class="text-cyan-400">browse</span>
                    </p>
                    <p class="mt-1 text-xs text-slate-500">
                        .txt or .log files up to {{ maxSizeMb }}MB
                    </p>
                </template>
            </div>
            <input
                type="file"
                class="absolute inset-0 cursor-pointer opacity-0"
                :accept="accept"
                @change="handleFileInput"
            />
        </label>
        <p v-if="error" class="px-4 pb-3 text-center text-sm text-red-400">
            {{ error }}
        </p>
    </div>
</template>
