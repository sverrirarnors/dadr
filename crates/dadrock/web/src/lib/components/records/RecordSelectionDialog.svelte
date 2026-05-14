<script lang="ts">
  import { tick } from 'svelte';
  import type { RecordSummary } from '$lib/api';
  import { Badge, EmptyState, ListRow } from '$lib/components/ui';
  import {
    displayRecordStatus,
    filterRecords,
    recordStatusVariant,
    searchScopeOptions,
    statusOptions,
    type RecordSearchScope,
    type RecordStatusFilter,
  } from '$lib/records/filters';

  type StatusVariant = 'candidate' | 'accepted' | 'rejected' | 'superseded';

  type Props = {
    title: string;
    records: RecordSummary[];
    selectedIds?: string[];
    excludedIds?: string[];
    query: string;
    status: RecordStatusFilter;
    confirmLabel: (count: number) => string;
    onconfirm: (recordIds: string[]) => void;
    oncancel: () => void;
  };

  let {
    title,
    records,
    selectedIds = [],
    excludedIds = [],
    query = $bindable(),
    status = $bindable(),
    confirmLabel,
    onconfirm,
    oncancel,
  }: Props = $props();

  let scope = $state<RecordSearchScope>('all');
  let draftSelectedIds = $derived([...selectedIds]);
  let searchInput: HTMLInputElement | undefined;
  const excluded = $derived(new Set(excludedIds));
  const filteredRecords = $derived(
    filterRecords(records, { status, query, scope }).filter((record) => !excluded.has(record.id)),
  );
  const newSelectedIds = $derived(
    draftSelectedIds.filter((recordId) => !selectedIds.includes(recordId)),
  );

  $effect(() => {
    void tick().then(() => searchInput?.focus());
  });

  function recordSelected(recordId: string): boolean {
    return selectedIds.includes(recordId);
  }

  function draftSelected(recordId: string): boolean {
    return draftSelectedIds.includes(recordId);
  }

  function toggleDraftRecord(recordId: string, checked: boolean) {
    if (recordSelected(recordId)) return;
    if (checked && !draftSelectedIds.includes(recordId)) {
      draftSelectedIds = [...draftSelectedIds, recordId];
    } else if (!checked) {
      draftSelectedIds = draftSelectedIds.filter((candidate) => candidate !== recordId);
    }
  }
</script>

<div
  data-record-picker-dialog
  role="dialog"
  aria-modal="true"
  aria-labelledby="record-picker-title"
  tabindex="-1"
  class="fixed inset-0 z-50 flex items-center justify-center bg-fg/25 p-4 backdrop-blur-sm"
  onclick={(event) => {
    if (event.target === event.currentTarget) oncancel();
  }}
  onkeydown={(event) => {
    if (event.key === 'Escape') oncancel();
  }}
>
  <div class="flex h-[min(44rem,calc(100vh-2rem))] w-[min(56rem,calc(100vw-2rem))] min-h-0 flex-col rounded-md border border-border bg-bg shadow-xl">
    <header class="flex flex-wrap items-start justify-between gap-3 border-b border-border p-4">
      <div>
        <h2 id="record-picker-title" class="text-lg font-semibold">{title}</h2>
        <p class="mt-1 text-sm text-fg-muted">
          {newSelectedIds.length} new selected · {selectedIds.length} already selected
        </p>
      </div>
    </header>

    <div class="flex min-h-0 flex-1 flex-col">
      <div class="border-b border-border p-4">
        <label class="flex h-9 overflow-hidden rounded-md border border-border bg-bg focus-within:ring-2 focus-within:ring-accent">
          <span class="sr-only">Search records</span>
          <select
            aria-label="Search scope"
            bind:value={scope}
            class="w-24 border-r border-border bg-surface px-2 text-xs text-fg-muted focus:outline-none"
          >
            {#each searchScopeOptions as option (option.value)}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
          <input
            bind:this={searchInput}
            type="search"
            bind:value={query}
            aria-label="Search records"
            placeholder="Search title, path, tag, or id"
            class="min-w-0 flex-1 bg-bg px-3 text-sm text-fg placeholder:text-fg-subtle focus:outline-none"
          />
        </label>
        <div data-record-status-filter class="mt-3 flex overflow-x-auto rounded-md border border-border bg-bg p-0.5">
          {#each statusOptions as option (option.value)}
            <button
              type="button"
              class="h-7 shrink-0 rounded px-2.5 text-xs font-medium text-fg-muted transition-colors hover:text-fg"
              class:bg-surface={status === option.value}
              class:text-fg={status === option.value}
              aria-pressed={status === option.value}
              onclick={() => (status = option.value)}
            >
              {option.label}
            </button>
          {/each}
        </div>
      </div>

      {#if filteredRecords.length}
        <ul data-record-picker-results class="min-h-0 flex-1 divide-y divide-border overflow-auto">
          {#each filteredRecords as record (record.id)}
            <li class="bg-bg transition-colors hover:bg-surface">
              <ListRow class="grid gap-3 px-2 py-2 md:grid-cols-[1fr_auto]">
                <label class="flex min-w-0 items-start gap-3">
                  <input
                    type="checkbox"
                    class="mt-1 h-4 w-4 accent-accent disabled:opacity-50"
                    aria-label={`Select ${record.title}`}
                    checked={draftSelected(record.id)}
                    disabled={recordSelected(record.id)}
                    onchange={(event) => toggleDraftRecord(record.id, event.currentTarget.checked)}
                  />
                  <span class="min-w-0">
                    <span class="flex min-w-0 items-center gap-2">
                      <span class="truncate font-medium text-fg">{record.title}</span>
                      <Badge variant={recordStatusVariant(record) as StatusVariant}>{displayRecordStatus(record)}</Badge>
                    </span>
                    <span class="mt-1 block truncate text-xs text-fg-subtle">{record.path}</span>
                  </span>
                </label>
                {#if recordSelected(record.id)}
                  <span class="self-center rounded bg-surface-2 px-1.5 py-0.5 text-[11px] text-fg-muted">
                    Selected
                  </span>
                {/if}
              </ListRow>
            </li>
          {/each}
        </ul>
      {:else}
        <div class="min-h-0 flex-1 p-4">
          <EmptyState>No records match this search.</EmptyState>
        </div>
      {/if}
    </div>

    <footer class="flex flex-wrap items-center justify-end gap-2 border-t border-border p-4">
      <button
        type="button"
        class="inline-flex h-8 items-center rounded-md border border-border bg-bg px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface hover:text-fg"
        onclick={oncancel}
      >
        Cancel
      </button>
      <button
        type="button"
        class="inline-flex h-8 items-center rounded-md bg-accent px-3 text-sm font-medium text-accent-fg transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
        disabled={newSelectedIds.length === 0}
        onclick={() => onconfirm(newSelectedIds)}
      >
        {confirmLabel(newSelectedIds.length)}
      </button>
    </footer>
  </div>
</div>
