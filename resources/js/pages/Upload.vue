<script setup lang="ts">
import { Head, useForm } from '@inertiajs/vue3';
import { store } from '@/actions/App/Http/Controllers/CombatLogController';

import FileDropZone from '@/components/ui/FileDropZone.vue';

defineProps<{
    maxUploadSizeMb: number;
}>();

const form = useForm({
    log_file: null as File | null,
});

function onFileSelected(file: File) {
    form.clearErrors('log_file');
    form.log_file = file;
}

function onFileError(message: string) {
    form.log_file = null;
    form.setError('log_file', message);
}

function submit() {
    form.submit(store());
}
</script>

<template>
    <Head title="Combat Log Analyzer">
        <meta
            name="description"
            content="Upload and analyze EVE Online combat logs. DPS graphs, damage breakdowns, logistics, and detailed pilot statistics."
        />
        <meta property="og:type" content="website" />
        <meta property="og:site_name" content="Combat Log Analyzer" />
        <meta property="og:title" content="Combat Log Analyzer" />
        <meta
            property="og:description"
            content="Upload and analyze EVE Online combat logs. DPS graphs, damage breakdowns, logistics, and detailed pilot statistics."
        />
        <meta name="twitter:card" content="summary" />
        <meta name="twitter:title" content="Combat Log Analyzer" />
        <meta
            name="twitter:description"
            content="Upload and analyze EVE Online combat logs. DPS graphs, damage breakdowns, logistics, and detailed pilot statistics."
        />
    </Head>
    <div
        class="flex min-h-screen flex-col items-center justify-center bg-slate-950 p-6"
    >
        <!-- Subtle grid background -->
        <div
            class="pointer-events-none fixed inset-0 opacity-[0.03]"
            style="
                background-image:
                    linear-gradient(
                        rgba(148, 163, 184, 0.5) 1px,
                        transparent 1px
                    ),
                    linear-gradient(
                        90deg,
                        rgba(148, 163, 184, 0.5) 1px,
                        transparent 1px
                    );
                background-size: 40px 40px;
            "
        />

        <div class="relative w-full max-w-lg">
            <div class="mb-8 text-center">
                <h1
                    class="font-mono text-3xl font-bold tracking-tight text-slate-100"
                >
                    COMBAT LOG
                    <span class="text-cyan-400">ANALYZER</span>
                </h1>
                <p class="mt-2 text-sm text-slate-500">
                    Upload your EVE Online combat log to analyze DPS, damage
                    breakdowns, logistics, and more.
                </p>
            </div>

            <form @submit.prevent="submit">
                <FileDropZone
                    accept=".txt,.log"
                    :max-size-mb="maxUploadSizeMb"
                    :error="form.errors.log_file"
                    @file="onFileSelected"
                    @error="onFileError"
                />

                <div v-if="form.progress" class="mt-4">
                    <div class="h-1 overflow-hidden rounded-full bg-slate-800">
                        <div
                            class="h-full rounded-full bg-cyan-400 shadow-[0_0_8px_rgba(34,211,238,0.5)] transition-all duration-300"
                            :style="{
                                width: `${form.progress.percentage}%`,
                            }"
                        />
                    </div>
                    <p
                        class="mt-1 text-center font-mono text-xs text-slate-500"
                    >
                        UPLOADING... {{ form.progress.percentage }}%
                    </p>
                </div>

                <button
                    type="submit"
                    :disabled="!form.log_file || form.processing"
                    class="mt-4 w-full rounded border border-cyan-500/30 bg-cyan-500/10 px-4 py-3 font-mono text-sm font-medium tracking-wider text-cyan-400 uppercase transition-all hover:border-cyan-400/50 hover:bg-cyan-500/20 hover:shadow-[0_0_20px_rgba(34,211,238,0.1)] disabled:cursor-not-allowed disabled:opacity-30"
                >
                    <span v-if="form.processing">ANALYZING...</span>
                    <span v-else>ANALYZE COMBAT LOG</span>
                </button>
            </form>
        </div>
    </div>
</template>
