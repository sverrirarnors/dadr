<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { createQuery } from '@tanstack/svelte-query';
  import { onMount } from 'svelte';
  import { createApi, type RecordSummary } from '$lib/api';
  import TagFacet from '$lib/components/ledger/TagFacet.svelte';
  import RecordDetailPanel from '$lib/components/records/RecordDetailPanel.svelte';
  import InitializePrompt from '$lib/components/shell/InitializePrompt.svelte';
  import { Alert, Badge, EmptyState, ListRow, TextInput } from '$lib/components/ui';
  import { PaneHeader, PaneToolbar, ResizableWorkbenchSplit, WorkbenchPane } from '$lib/components/workbench';
  import {
    ledgerCounts,
    ledgerRecords,
    ledgerSubtitle,
    readLedgerFilters,
    type LedgerFilters,
  } from '$lib/ledger/filters';
  import { ledgerRecordUrl, ledgerUrl } from '$lib/navigation/routes';
  import { displayRecordStatus, recordStatusVariant } from '$lib/records/filters';

  type StatusVariant = 'candidate' | 'accepted' | 'rejected' | 'superseded';

  const api = createApi();
  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  const filters = $derived(readLedgerFilters(page.url.searchParams));
  const records = $derived(snapshot.data?.records ?? []);
  const rows = $derived(ledgerRecords(records, filters));
  const counts = $derived(ledgerCounts(records));
  const selectedId = $derived(page.url.searchParams.get('id') ?? rows[0]?.id ?? '');

  // Below md the detail pane is hidden, so rows open the full-page record view.
  let wide = $state(false);

  onMount(() => {
    const query = window.matchMedia('(min-width: 768px)');
    const update = () => {
      wide = query.matches;
    };
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  });

  function updateFilters(patch: Partial<LedgerFilters>) {
    const next: LedgerFilters = { ...filters, ...patch };
    const currentId = page.url.searchParams.get('id');
    const keepId = currentId && ledgerRecords(records, next).some((record) => record.id === currentId);
    void goto(
      resolve(ledgerUrl({ q: next.q, tag: next.tag, history: next.history, id: keepId ? currentId : null })),
      { replaceState: true, noScroll: true, keepFocus: true },
    );
  }

  function rowHref(record: RecordSummary) {
    if (!wide) return ledgerRecordUrl(record.id);
    return ledgerUrl({ q: filters.q, tag: filters.tag, history: filters.history, id: record.id });
  }
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden">
  <h1 class="sr-only">Ledger</h1>

  {#if snapshot.isPending}
    <p class="p-4 text-sm text-fg-muted">Loading...</p>
  {:else if snapshot.isError}
    <Alert class="m-2" variant="danger" title="Failed to load snapshot">{snapshot.error.message}</Alert>
  {:else if !snapshot.data.project.initialized}
    <InitializePrompt cwd={snapshot.data.project.cwd} />
  {:else}
    <ResizableWorkbenchSplit splitId="ledger" handleLabel="Resize ledger panes">
      {#snippet left()}
        <WorkbenchPane framed data-ledger-list-pane class="flex min-h-0 min-w-0 flex-col">
          <PaneHeader title="Ledger" subtitle={ledgerSubtitle(counts, filters.history)} />

          <PaneToolbar>
            <TextInput
              type="search"
              aria-label="Search ledger"
              placeholder="Search title, path, tag, or id"
              class="min-w-0 flex-1"
              value={filters.q}
              oninput={(event) => updateFilters({ q: event.currentTarget.value })}
            />
            <label class="flex h-9 shrink-0 cursor-pointer items-center gap-1.5 text-xs text-fg-muted">
              <input
                type="checkbox"
                class="h-3.5 w-3.5 accent-accent"
                checked={filters.history}
                onchange={(event) => updateFilters({ history: event.currentTarget.checked })}
              />
              <span>Show history</span>
            </label>
          </PaneToolbar>

          <TagFacet
            tags={snapshot.data.tags}
            selected={filters.tag}
            onselect={(tag) => updateFilters({ tag })}
          />

          <ul data-ledger-list class="min-h-0 flex-1 divide-y divide-border overflow-auto">
            {#each rows as record (record.id)}
              <li class="bg-bg transition-colors hover:bg-surface">
                <a
                  class="block px-1.5 py-1"
                  href={resolve(rowHref(record))}
                  aria-current={selectedId === record.id ? 'page' : undefined}
                >
                  <ListRow active={selectedId === record.id} class="flex items-center gap-2 py-1.5">
                    {#if filters.history}
                      <Badge variant={recordStatusVariant(record) as StatusVariant} mono class="uppercase">
                        {displayRecordStatus(record).slice(0, 3)}
                      </Badge>
                    {/if}
                    <span class="min-w-0 flex-1 truncate text-sm font-medium text-fg">{record.title}</span>
                    <span class="shrink-0 font-mono text-[11px] text-fg-subtle">{record.id_prefix}</span>
                  </ListRow>
                </a>
              </li>
            {:else}
              <li class="p-3">
                <EmptyState>
                  {#if filters.q || filters.tag}
                    No decisions match. Clear the search or tag filter{filters.history ? '' : ', or show history'}.
                  {:else if filters.history}
                    No decisions yet. Accept a candidate in the Inbox to record one.
                  {:else}
                    No current decisions. Show history to see superseded and rejected ones.
                  {/if}
                </EmptyState>
              </li>
            {/each}
          </ul>
        </WorkbenchPane>
      {/snippet}

      {#snippet right()}
        <WorkbenchPane framed data-record-detail-pane class="hidden min-h-0 min-w-0 flex-col p-4 md:flex md:p-5">
          <RecordDetailPanel id={selectedId} surface="ledger" embedded />
        </WorkbenchPane>
      {/snippet}
    </ResizableWorkbenchSplit>
  {/if}
</div>
