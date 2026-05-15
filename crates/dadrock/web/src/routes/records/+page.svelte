<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { FilePlus2, RefreshCw, Sparkles } from 'lucide-svelte';
  import { onMount } from 'svelte';
  import { createApi } from '$lib/api';
  import RecordDetailPanel from '$lib/components/records/RecordDetailPanel.svelte';
  import { ActionMenu, actionMenuItemClass, Alert, Badge, Button, EmptyState, ListRow } from '$lib/components/ui';
  import { PaneHeader, PaneToolbar, ResizableWorkbenchSplit, WorkbenchPane } from '$lib/components/workbench';
  import type { RecordSummary } from '$lib/api';
  import { toasts } from '$lib/stores/toast.svelte';
  import {
    displayRecordStatus,
    filterRecords,
    recordStatusVariant,
    searchScopeOptions,
    statusOptions,
    type RecordSearchScope,
    type RecordStatusFilter,
  } from '$lib/records/filters';
  import { applyMutationResponse } from '$lib/query/mutation-cache';

  const api = createApi();
  const queryClient = useQueryClient();

  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  const initialize = createMutation(() => ({
    mutationFn: () => api.initializeProject(),
    onSuccess: async (response) => {
      toasts.push({ severity: 'success', message: response.message });
      await applyMutationResponse(queryClient, response);
    },
  }));

  type StatusVariant = 'candidate' | 'accepted' | 'rejected' | 'superseded';
  const status = $derived(readStatus(page.url.searchParams.get('status')));
  const query = $derived(page.url.searchParams.get('q') ?? '');
  const scope = $derived(readScope(page.url.searchParams.get('scope')));
  const filteredRecords = $derived(
    snapshot.data ? filterRecords(snapshot.data.records, { status, query, scope }) : [],
  );
  const selectedId = $derived(page.url.searchParams.get('id') ?? filteredRecords[0]?.id ?? '');
  let wide = $state(false);

  onMount(() => {
    const query = window.matchMedia('(min-width: 1024px)');
    const update = () => {
      wide = query.matches;
    };
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  });

  function readStatus(value: string | null): RecordStatusFilter {
    return statusOptions.some((option) => option.value === value)
      ? (value as RecordStatusFilter)
      : 'all';
  }

  function readScope(value: string | null): RecordSearchScope {
    return searchScopeOptions.some((option) => option.value === value)
      ? (value as RecordSearchScope)
      : 'all';
  }

  function setParam(name: 'q' | 'status' | 'scope', value: string) {
    const next = new URL(page.url);
    next.searchParams.delete('id');
    next.searchParams.delete('tab');
    if (!value || value === 'all') {
      next.searchParams.delete(name);
    } else {
      next.searchParams.set(name, value);
    }
    const target = (next.search ? `/records${next.search}` : '/records') as
      | '/records'
      | `/records?${string}`;
    goto(resolve(target), {
      replaceState: true,
      noScroll: true,
      keepFocus: true,
    });
  }

  function recordHref(record: RecordSummary) {
    if (!wide) {
      return `/records/${record.id}` as `/records/${string}`;
    }
    const next = new URL(page.url);
    next.searchParams.set('id', record.id);
    next.searchParams.delete('tab');
    const target = (next.search ? `/records${next.search}` : '/records') as
      | '/records'
      | `/records?${string}`;
    return target;
  }

</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden">
  <h1 class="sr-only">Records</h1>

  {#if snapshot.isPending}
    <p class="mt-6 text-sm text-fg-muted">Loading...</p>
  {:else if snapshot.isError}
    <Alert class="m-2" variant="danger" title="Failed to load snapshot">{snapshot.error.message}</Alert>
  {:else if !snapshot.data.project.initialized}
    <div class="flex min-h-0 flex-1 items-center justify-center overflow-auto p-4">
      <section
        aria-labelledby="initialize-project-title"
        class="w-full max-w-2xl rounded-md border border-border bg-surface p-5 shadow-sm"
      >
        <div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div class="min-w-0">
            <h2 id="initialize-project-title" class="text-base font-semibold text-fg">
              Initialize Dadrock here?
            </h2>
            <p class="mt-2 text-sm text-fg-muted">
              No Dadrock project was found in
              <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">{snapshot.data.project.cwd}</code>.
            </p>
          </div>
          <Button
            type="button"
            variant="outline"
            size="lg"
            class="shrink-0"
            disabled={initialize.isPending}
            onclick={() => initialize.mutate()}
          >
            <RefreshCw class={`h-4 w-4 ${initialize.isPending ? 'animate-spin' : ''}`} aria-hidden="true" />
            <span>{initialize.isPending ? 'Initializing' : 'Initialize here'}</span>
          </Button>
        </div>

        <div class="mt-4 space-y-3 text-sm leading-6 text-fg-muted">
          <p>
            This creates the local Dadrock project structure in the current directory:
            <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">dadr/accepted/</code>,
            <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">dadr/candidates/</code>,
            <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">dadr/rejected/</code>,
            <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">dadr/templates/</code>,
            <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">dadr/tasks/</code>,
            and
            <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">dadr/runs/</code>.
          </p>
          <p>
            Choose this only if
            <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">{snapshot.data.project.cwd}</code>
            is the project Dadrock should manage.
          </p>
        </div>

        {#if initialize.isError}
          <Alert class="mt-4" variant="danger" title="Initialization failed">{initialize.error.message}</Alert>
        {/if}
      </section>
    </div>
  {:else}
    <ResizableWorkbenchSplit splitId="records" handleLabel="Resize records panes">
      {#snippet left()}
      <WorkbenchPane framed class="min-h-0 min-w-0">
        <div class="flex h-full min-h-0 flex-col">
          <PaneHeader title="Records" subtitle={`Showing ${filteredRecords.length} of ${snapshot.data.records.length}`}>
            {#snippet actions()}
              <ActionMenu label="New">
                <a role="menuitem" class={actionMenuItemClass} href={resolve('/candidates/new')}>
                  <FilePlus2 class="h-3.5 w-3.5" aria-hidden="true" />
                  <span>New</span>
                </a>
                <a role="menuitem" class={actionMenuItemClass} href={resolve('/candidates/new?mode=agent')}>
                  <Sparkles class="h-3.5 w-3.5" aria-hidden="true" />
                  <span>New with agent</span>
                </a>
              </ActionMenu>
            {/snippet}
          </PaneHeader>
          <PaneToolbar class="flex-col items-stretch">
            <label class="flex h-9 overflow-hidden rounded-md border border-border bg-bg focus-within:ring-2 focus-within:ring-accent">
              <span class="sr-only">Search records</span>
              <select
                aria-label="Search scope"
                value={scope}
                class="w-24 border-r border-border bg-surface px-2 text-xs text-fg-muted focus:outline-none"
                onchange={(event) => setParam('scope', event.currentTarget.value)}
              >
                {#each searchScopeOptions as option (option.value)}
                  <option value={option.value}>{option.label}</option>
                {/each}
              </select>
              <input
                type="search"
                value={query}
                placeholder="Search title, path, tag, or id"
                class="min-w-0 flex-1 bg-bg px-3 text-sm text-fg placeholder:text-fg-subtle focus:outline-none"
                oninput={(event) => setParam('q', event.currentTarget.value)}
              />
            </label>
            <div data-record-status-filter class="mt-3 flex overflow-x-auto rounded-md border border-border bg-bg p-0.5">
              {#each statusOptions as option (option.value)}
                <button
                  type="button"
                  class="h-7 shrink-0 rounded px-2.5 text-xs font-medium text-fg-muted transition-colors hover:text-fg"
                  class:bg-surface={status === option.value}
                  class:text-fg={status === option.value}
                  onclick={() => setParam('status', option.value)}
                >
                  {option.label}
                </button>
              {/each}
            </div>
          </PaneToolbar>

          <ul data-records-list-pane class="min-h-0 flex-1 divide-y divide-border overflow-auto">
            {#each filteredRecords as record (record.id)}
              <li class="bg-bg transition-colors hover:bg-surface">
                <a
                  class="block px-1.5 py-1.5"
                  href={resolve(recordHref(record))}
                  aria-current={selectedId === record.id ? 'page' : undefined}
                >
                  <ListRow active={selectedId === record.id} class="py-2">
                    <div class="flex items-center gap-2">
                      <Badge variant={recordStatusVariant(record) as StatusVariant} mono class="uppercase">
                      {displayRecordStatus(record).slice(0, 3)}
                      </Badge>
                      <span class="min-w-0 flex-1 truncate text-sm font-medium text-fg">{record.title}</span>
                    </div>
                  </ListRow>
                </a>
              </li>
            {:else}
              <li class="p-3">
                <EmptyState>No records match the current filters.</EmptyState>
              </li>
            {/each}
          </ul>
        </div>
      </WorkbenchPane>
      {/snippet}

      {#snippet right()}
      <WorkbenchPane framed data-record-detail-pane class="hidden min-h-0 min-w-0 flex-col p-4 md:flex md:p-5">
        <RecordDetailPanel id={selectedId} showBackLink={false} embedded />
      </WorkbenchPane>
      {/snippet}
    </ResizableWorkbenchSplit>
  {/if}
</div>
