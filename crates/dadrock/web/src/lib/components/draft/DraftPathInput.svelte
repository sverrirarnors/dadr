<script lang="ts">
  import { tick } from 'svelte';
  import { FolderOpen, FileText } from 'lucide-svelte';
  import { Button } from '$lib/components/ui';

  type Props = {
    selectedPaths: string[];
    onadd: (path: string) => void;
  };

  let { selectedPaths, onadd }: Props = $props();

  let value = $state('');
  let input = $state<HTMLInputElement | undefined>();
  let fileInput = $state<HTMLInputElement | undefined>();
  let folderInput = $state<HTMLInputElement | undefined>();

  const trimmed = $derived(value.trim());
  const canAdd = $derived(Boolean(trimmed) && !selectedPaths.includes(trimmed));

  $effect(() => {
    void tick().then(() => input?.focus());
  });

  $effect(() => {
    if (!folderInput) return;
    folderInput.setAttribute('webkitdirectory', '');
    folderInput.setAttribute('directory', '');
  });

  function addCurrent() {
    if (!canAdd) return;
    onadd(trimmed);
    value = '';
    input?.focus();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    addCurrent();
  }

  function handleFolderSelection(event: Event) {
    const target = event.currentTarget as HTMLInputElement;
    const roots = unique(
      Array.from(target.files ?? [])
        .map((file) => relativePath(file).split('/')[0]?.trim() ?? '')
        .filter(Boolean),
    );
    if (roots.length) {
      for (const root of roots) onadd(root);
    } else {
      addFiles(target.files);
    }
    target.value = '';
  }

  function handleFileSelection(event: Event) {
    const target = event.currentTarget as HTMLInputElement;
    addFiles(target.files);
    target.value = '';
  }

  function addFiles(files: FileList | null) {
    for (const file of Array.from(files ?? [])) {
      const path = relativePath(file);
      if (path) onadd(path);
    }
  }

  function relativePath(file: File): string {
    return ((file as File & { webkitRelativePath?: string }).webkitRelativePath || file.name).trim();
  }

  function unique(values: string[]): string[] {
    return values.filter((value, index) => values.indexOf(value) === index);
  }
</script>

<div data-draft-path-input class="rounded-md border border-border bg-surface p-2">
  <div class="flex gap-2">
    <input
      bind:this={input}
      bind:value
      type="text"
      aria-label="Path"
      placeholder="analysis or analysis/model.R"
      autocomplete="off"
      spellcheck="false"
      class="h-8 min-w-0 flex-1 rounded-md border border-border bg-bg px-2.5 font-mono text-xs text-fg placeholder:text-fg-subtle focus:outline-none focus:ring-2 focus:ring-accent"
      onkeydown={handleKeydown}
    />
    <Button type="button" variant="outline" size="sm" disabled={!canAdd} onclick={addCurrent}>Add</Button>
  </div>
  <div class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-fg-muted">
    <button
      type="button"
      class="inline-flex items-center gap-1 rounded hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      onclick={() => folderInput?.click()}
    >
      <FolderOpen class="h-3.5 w-3.5" aria-hidden="true" />
      <span>Choose a folder</span>
    </button>
    <button
      type="button"
      class="inline-flex items-center gap-1 rounded hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      onclick={() => fileInput?.click()}
    >
      <FileText class="h-3.5 w-3.5" aria-hidden="true" />
      <span>Choose files</span>
    </button>
    <span class="text-fg-subtle">Paths are relative to the project root.</span>
    <input
      bind:this={folderInput}
      class="sr-only"
      type="file"
      multiple
      tabindex="-1"
      aria-label="Select local folder from system dialog"
      onchange={handleFolderSelection}
    />
    <input
      bind:this={fileInput}
      class="sr-only"
      type="file"
      multiple
      tabindex="-1"
      aria-label="Select local files from system dialog"
      onchange={handleFileSelection}
    />
  </div>
</div>
