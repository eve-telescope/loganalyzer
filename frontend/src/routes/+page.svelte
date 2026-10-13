<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';

  import FileDropZone from '#lib/components/file-drop-zone.svelte';
  import PageHead from '#lib/components/page-head.svelte';
  import { uploadHead } from '#lib/seo.ts';
  import { UploadForm } from '#lib/upload.svelte.ts';

  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  // svelte-ignore state_referenced_locally
  const form = new UploadForm(data.maxUploadSizeMb);

  function onFileSelected(file: File) {
    form.clearErrors();
    form.logFile = file;
  }

  function onFileError(message: string) {
    form.logFile = null;
    form.setError(message);
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    const redirect = await form.submit();
    if (redirect) {
      await goto(redirect);
    }
  }
</script>

<PageHead head={uploadHead(data.name, page.url)} />

<div class="flex min-h-screen flex-col items-center justify-center bg-zinc-950 p-6">
  <!-- Subtle grid background -->
  <div
    class="pointer-events-none fixed inset-0 opacity-[0.03]"
    style="
      background-image:
        linear-gradient(rgba(148, 163, 184, 0.5) 1px, transparent 1px),
        linear-gradient(90deg, rgba(148, 163, 184, 0.5) 1px, transparent 1px);
      background-size: 40px 40px;
    "
  ></div>

  <a
    href="https://github.com/eve-telescope/loganalyzer"
    target="_blank"
    rel="noopener noreferrer"
    title="View source on GitHub"
    class="fixed top-5 right-5 text-zinc-500 transition-colors hover:text-amber-300"
  >
    <svg class="h-5 w-5" fill="currentColor" viewBox="0 0 24 24">
      <path
        fill-rule="evenodd"
        clip-rule="evenodd"
        d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0 1 12 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0 0 22 12.017C22 6.484 17.522 2 12 2Z"
      />
    </svg>
  </a>

  <div class="relative w-full max-w-lg">
    <div class="mb-8 text-center">
      <h1 class="font-mono text-3xl font-bold tracking-tight text-zinc-100">
        COMBAT LOG
        <span class="text-amber-400">ANALYZER</span>
      </h1>
      <p class="mt-2 text-sm text-zinc-500">
        Upload your EVE Online combat log to analyze DPS, damage breakdowns, logistics, and more.
      </p>
    </div>

    <form onsubmit={submit}>
      <FileDropZone
        accept=".txt,.log"
        maxSizeMb={data.maxUploadSizeMb}
        error={form.errors.log_file}
        onFile={onFileSelected}
        onError={onFileError}
      />

      {#if form.progress}
        <div class="mt-4">
          <div class="h-1 overflow-hidden rounded-full bg-zinc-800">
            <div
              class="h-full rounded-full bg-amber-400 shadow-[0_0_8px_rgba(34,211,238,0.5)] transition-all duration-300"
              style:width="{form.progress.percentage}%"
            ></div>
          </div>
          <p class="mt-1 text-center font-mono text-xs text-zinc-500">
            UPLOADING... {form.progress.percentage}%
          </p>
        </div>
      {/if}

      <button
        type="submit"
        disabled={!form.logFile || form.processing}
        class="mt-4 w-full rounded border border-amber-500/30 bg-amber-500/10 px-4 py-3 font-mono text-sm font-medium tracking-wider text-amber-400 uppercase transition-all hover:border-amber-400/50 hover:bg-amber-500/20 hover:shadow-[0_0_20px_rgba(34,211,238,0.1)] disabled:cursor-not-allowed disabled:opacity-30"
      >
        {#if form.processing}
          <span>ANALYZING...</span>
        {:else}
          <span>ANALYZE COMBAT LOG</span>
        {/if}
      </button>
    </form>
  </div>
</div>
