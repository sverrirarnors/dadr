<script lang="ts">
  import { FolderOpen, Paperclip, X } from 'lucide-svelte';
  import type { RecordSummary } from '$lib/api';
  import { Badge } from '$lib/components/ui';
  import { displayRecordStatus, recordStatusVariant } from '$lib/records/filters';
  import { composer } from '$lib/stores/composer.svelte';
  import DraftPathInput from './DraftPathInput.svelte';

  type StatusVariant = 'candidate' | 'accepted' | 'rejected' | 'superseded';

  type Props = {
    records: RecordSummary[];
    /** A record that stays attached: shown without a remove button. */
    pinnedId?: string | undefined;
    canAttachRecords: boolean;
    onattachrecords: () => void;
  };

  let { records, pinnedId, canAttachRecords, onattachrecords }: Props = $props();

  let pathInputOpen = $state(false);

  const hasAttachments = $derived(composer.paths.length > 0 || composer.recordIds.length > 0);

  function recordFor(id: string): RecordSummary | undefined {
    return records.find((record) => record.id === id);
  }

  function addPath(path: string) {
    if (composer.paths.includes(path)) return;
    composer.paths = [...composer.paths, path];
  }

  function removePath(path: string) {
    composer.paths = composer.paths.filter((candidate) => candidate !== path);
  }

  function removeRecord(id: string) {
    composer.recordIds = composer.recordIds.filter((candidate) => candidate !== id);
  }

  const chipClass =
    'inline-flex h-7 max-w-full items-center gap-1.5 rounded-md border border-border bg-surface pl-2 pr-1 text-xs text-fg';
  const removeClass =
    'inline-flex h-5 w-5 shrink-0 items-center justify-center rounded text-fg-subtle hover:bg-surface-2 hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent';
  const attachClass =
    'inline-flex h-7 items-center gap-1.5 rounded-md px-2 text-xs text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent';
</script>

<div data-draft-attachments class="space-y-2">
  {#if hasAttachments}
    <ul class="flex flex-wrap gap-1.5" aria-label="Attachments">
      {#each composer.paths as path (path)}
        <li class={chipClass} data-draft-path-chip>
          <FolderOpen class="h-3.5 w-3.5 shrink-0 text-fg-subtle" aria-hidden="true" />
          <span class="truncate font-mono">{path}</span>
          <button type="button" class={removeClass} aria-label={`Remove ${path}`} onclick={() => removePath(path)}>
            <X class="h-3 w-3" aria-hidden="true" />
          </button>
        </li>
      {/each}
      {#each composer.recordIds as id (id)}
        {@const record = recordFor(id)}
        {@const title = record?.title ?? id}
        <li class={chipClass} data-draft-record-chip>
          <span class="truncate">{title}</span>
          {#if record}
            <Badge variant={recordStatusVariant(record) as StatusVariant}>{displayRecordStatus(record)}</Badge>
          {/if}
          {#if id !== pinnedId}
            <button type="button" class={removeClass} aria-label={`Remove ${title}`} onclick={() => removeRecord(id)}>
              <X class="h-3 w-3" aria-hidden="true" />
            </button>
          {:else}
            <span class="w-1" aria-hidden="true"></span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  <div class="flex flex-wrap items-center gap-1">
    <button
      type="button"
      class={attachClass}
      class:bg-surface-2={pathInputOpen}
      class:text-fg={pathInputOpen}
      aria-expanded={pathInputOpen}
      onclick={() => (pathInputOpen = !pathInputOpen)}
    >
      <Paperclip class="h-3.5 w-3.5" aria-hidden="true" />
      <span>Attach files or folders</span>
    </button>
    {#if canAttachRecords}
      <button type="button" class={attachClass} onclick={onattachrecords}>
        <Paperclip class="h-3.5 w-3.5" aria-hidden="true" />
        <span>Attach records</span>
      </button>
    {/if}
  </div>

  {#if pathInputOpen}
    <DraftPathInput selectedPaths={composer.paths} onadd={addPath} />
  {/if}
</div>
