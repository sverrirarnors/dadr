<script lang="ts">
  type PickerKind = 'file' | 'directory';

  type Props = {
    kind: PickerKind;
    value: string;
    selectedPaths: string[];
    onadd: (path: string) => void;
    onremove: (path: string) => void;
  };

  let { kind, value = $bindable(), selectedPaths, onadd, onremove }: Props = $props();
  let systemInput = $state<HTMLInputElement>();

  const trimmedValue = $derived(value.trim());
  const canAdd = $derived(Boolean(trimmedValue) && !selectedPaths.includes(trimmedValue));
  const inputLabel = $derived(kind === 'file' ? 'File path' : 'Directory path');
  const chooseLabel = $derived(kind === 'file' ? 'Select local files' : 'Select local folder');
  const useLabel = $derived(kind === 'file' ? 'Use file path' : 'Use directory path');
  const systemLabel = $derived(
    kind === 'file' ? 'Select local files from system dialog' : 'Select local folder from system dialog',
  );
  const placeholder = $derived(kind === 'file' ? 'analysis/model.R' : 'analysis or crates/dadrock');
  const emptyLabel = $derived(kind === 'file' ? 'No files selected.' : 'No directories selected.');

  $effect(() => {
    if (!systemInput || kind !== 'directory') return;
    systemInput.setAttribute('webkitdirectory', '');
    systemInput.setAttribute('directory', '');
  });

  function addCurrent() {
    if (!canAdd) return;
    onadd(trimmedValue);
    value = '';
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    addCurrent();
  }

  function openSystemPicker() {
    systemInput?.click();
  }

  function handleSystemSelection(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    if (kind === 'directory') {
      addSelectedDirectoryRoots(input.files);
      input.value = '';
      return;
    }
    addSelectedFilePaths(input.files);
    input.value = '';
  }

  function addSelectedDirectoryRoots(files: FileList | null) {
    const roots = unique(
      Array.from(files ?? [])
        .map((file) => selectedFilePath(file))
        .map((path) => path.split('/')[0]?.trim() ?? '')
        .filter(Boolean),
    );
    if (roots.length) {
      for (const root of roots) onadd(root);
      return;
    }
    addSelectedFilePaths(files);
  }

  function addSelectedFilePaths(files: FileList | null) {
    for (const file of Array.from(files ?? [])) {
      const path = selectedFilePath(file);
      if (path) onadd(path);
    }
  }

  function selectedFilePath(file: File): string {
    return ((file as File & { webkitRelativePath?: string }).webkitRelativePath || file.name).trim();
  }

  function unique(values: string[]): string[] {
    return Array.from(new Set(values));
  }
</script>

<div data-task-path-picker class="space-y-3">
  <div class="flex flex-wrap gap-2">
    <button
      type="button"
      class="h-9 rounded-md border border-border bg-surface px-3 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
      onclick={openSystemPicker}
    >
      {chooseLabel}
    </button>
    <input
      bind:this={systemInput}
      class="sr-only"
      type="file"
      multiple
      aria-label={systemLabel}
      onchange={handleSystemSelection}
    />
  </div>

  <div class="grid gap-2 md:grid-cols-[1fr_auto]">
    <label class="block">
      <span class="mb-1 block text-sm font-medium text-fg">{inputLabel}</span>
      <input
        type="search"
        class="h-9 w-full rounded-md border border-border bg-bg px-3 font-mono text-xs text-fg placeholder:text-fg-subtle focus:outline-none focus:ring-2 focus:ring-accent"
        aria-label={inputLabel}
        {placeholder}
        bind:value
        onkeydown={handleKeydown}
      />
    </label>
    <button
      type="button"
      class="mt-6 h-9 rounded-md border border-border bg-surface px-3 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg disabled:cursor-not-allowed disabled:opacity-50"
      disabled={!canAdd}
      onclick={addCurrent}
    >
      {useLabel}
    </button>
  </div>

  {#if selectedPaths.length}
    <div class="divide-y divide-border border-y border-border">
      {#each selectedPaths as path (path)}
        <div class="flex items-center justify-between gap-2 py-2 text-sm">
          <div class="min-w-0 truncate font-mono text-xs text-fg">{path}</div>
          <button
            type="button"
            class="shrink-0 rounded border border-border bg-surface px-2 py-1 text-xs text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
            aria-label={`Remove path ${path}`}
            onclick={() => onremove(path)}
          >
            Remove
          </button>
        </div>
      {/each}
    </div>
  {:else}
    <p class="text-sm text-fg-muted">{emptyLabel}</p>
  {/if}
</div>
