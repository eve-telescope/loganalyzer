<script lang="ts">
  import { fitsLimit, tooLargeMessage } from '#lib/upload.ts';

  let {
    accept,
    error,
    maxSizeMb,
    onFile,
    onError,
  }: {
    accept?: string;
    error?: string;
    maxSizeMb: number;
    onFile: (file: File) => void;
    onError: (message: string) => void;
  } = $props();

  let isDragging = $state(false);
  let fileName = $state<string | null>(null);

  function processFile(file: File) {
    if (!fitsLimit(file.size, maxSizeMb)) {
      fileName = null;
      onError(tooLargeMessage(maxSizeMb));
      return;
    }

    fileName = file.name;
    onFile(file);
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    isDragging = false;
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

<div
  class="relative rounded border-2 border-dashed transition-all duration-200 {isDragging
    ? 'border-amber-400 bg-amber-950/20 shadow-[0_0_20px_rgba(34,211,238,0.1)]'
    : error
      ? 'border-red-500/50 bg-red-950/10'
      : 'border-zinc-600 bg-zinc-800/40 hover:border-zinc-500 hover:bg-zinc-800/60'}"
  role="region"
  aria-label="Combat log drop zone"
  ondragover={(event) => {
    event.preventDefault();
    isDragging = true;
  }}
  ondragleave={(event) => {
    event.preventDefault();
    isDragging = false;
  }}
  ondrop={handleDrop}
>
  <label class="flex cursor-pointer flex-col items-center gap-3 p-10">
    <svg
      class="h-10 w-10 {isDragging ? 'text-amber-400' : 'text-zinc-500'}"
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
      {#if fileName}
        <p class="font-mono text-sm font-medium text-amber-400">
          {fileName}
        </p>
      {:else}
        <p class="text-sm font-medium text-zinc-300">
          Drop your combat log here or
          <span class="text-amber-400">browse</span>
        </p>
        <p class="mt-1 text-xs text-zinc-500">
          .txt or .log files up to {maxSizeMb}MB
        </p>
      {/if}
    </div>
    <input
      type="file"
      class="absolute inset-0 cursor-pointer opacity-0"
      {accept}
      onchange={handleFileInput}
    />
  </label>
  {#if error}
    <p class="px-4 pb-3 text-center text-sm text-red-400">
      {error}
    </p>
  {/if}
</div>
