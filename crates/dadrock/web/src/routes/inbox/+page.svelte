<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { ArrowLeft, ScanSearch } from 'lucide-svelte';
  import { onMount } from 'svelte';
  import { createApi } from '$lib/api';
  import InboxDetail from '$lib/components/inbox/InboxDetail.svelte';
  import InboxZero from '$lib/components/inbox/InboxZero.svelte';
  import InitializePrompt from '$lib/components/shell/InitializePrompt.svelte';
  import { Alert, Badge, Button, EmptyState, ListRow } from '$lib/components/ui';
  import { PaneHeader, ResizableWorkbenchSplit, WorkbenchPane } from '$lib/components/workbench';
  import { recordEditHref } from '$lib/diagnostics/actions';
  import { buildInboxGroups, countActionable, flattenInboxGroups, type InboxItem } from '$lib/inbox/items';
  import { actionSelector, inboxKeyAction, isTypingTarget, neighbourItemId } from '$lib/inbox/keyboard';
  import { kindBadge, recordLedgerHref } from '$lib/inbox/presentation';
  import { fetchInboxRuns, inboxRunsQueryKey, inboxRunsRefetchInterval, isActiveRun } from '$lib/inbox/queries';
  import { inboxUrl, type LedgerHref } from '$lib/navigation/routes';
  import { resolvedItems } from '$lib/stores/resolved.svelte';
  import { toasts } from '$lib/stores/toast.svelte';

  const api = createApi();
  const queryClient = useQueryClient();

  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  const runs = createQuery(() => ({
    queryKey: inboxRunsQueryKey,
    queryFn: () => fetchInboxRuns(api),
    enabled: snapshot.data?.project.initialized === true,
    refetchInterval: (query) => inboxRunsRefetchInterval(query.state.data),
  }));

  const groups = $derived(
    buildInboxGroups({
      snapshot: snapshot.data,
      runs: runs.data?.runs ?? [],
      results: runs.data?.results ?? {},
      resolved: resolvedItems.ids,
    }),
  );
  const items = $derived(flattenInboxGroups(groups));
  const itemIds = $derived(items.map((item) => item.id));
  const actionable = $derived(countActionable(groups));
  const subtitle = $derived(
    actionable === 0 ? 'Nothing waiting' : actionable === 1 ? '1 to deal with' : `${actionable} to deal with`,
  );
  const agentAvailable = $derived(snapshot.data?.settings.agent_available ?? false);

  const requestedId = $derived(page.url.searchParams.get('item'));
  const requestedItem = $derived(items.find((item) => item.id === requestedId));
  const selectedItem = $derived(requestedItem ?? items[0]);

  let wide = $state(false);
  const showNarrowDetail = $derived(!wide && requestedItem !== undefined);

  const conflictCheckActive = $derived(
    (runs.data?.runs ?? []).some((run) => run.task_id === 'find-conflicts' && isActiveRun(run)),
  );

  const checkConflicts = createMutation(() => ({
    mutationFn: () => api.runTask({ task_id: 'find-conflicts', input: { status_scope: 'live' } }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: inboxRunsQueryKey });
    },
    onError: (error) => {
      toasts.push({ severity: 'error', message: 'Could not start the conflict check', detail: error.message });
    },
  }));

  onMount(() => {
    const query = window.matchMedia('(min-width: 768px)');
    const update = () => {
      wide = query.matches;
    };
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  });

  $effect(() => {
    const id = selectedItem?.id;
    if (!id || !wide) return;
    document.querySelector(`[data-inbox-item="${CSS.escape(id)}"]`)?.scrollIntoView({ block: 'nearest' });
  });

  function select(itemId: string) {
    void goto(resolve(inboxUrl(itemId)), { replaceState: true, noScroll: true, keepFocus: true });
  }

  function handleRowClick(event: MouseEvent, item: InboxItem) {
    // Narrow screens navigate for real so Back returns to the list.
    if (!wide || event.defaultPrevented || event.button !== 0) return;
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    select(item.id);
  }

  function openHref(item: InboxItem): LedgerHref | null {
    if (item.kind === 'candidate') return recordLedgerHref(item.record);
    if (item.kind === 'issue' && item.record) return recordEditHref(item.record);
    if (item.kind === 'revision' && item.record) return recordLedgerHref(item.record);
    return null;
  }

  function handleKeydown(event: KeyboardEvent) {
    const action = inboxKeyAction(event);
    if (!action || !selectedItem) return;
    if (isTypingTarget(document.activeElement)) return;
    if (document.querySelector('[role="dialog"], dialog[open]')) return;

    if (action === 'next' || action === 'previous') {
      event.preventDefault();
      const target = neighbourItemId(itemIds, selectedItem.id, action === 'next' ? 1 : -1);
      if (target && target !== selectedItem.id) select(target);
      return;
    }

    if (action === 'open') {
      const href = openHref(selectedItem);
      if (!href) return;
      event.preventDefault();
      void goto(resolve(href));
      return;
    }

    const button = document.querySelector<HTMLButtonElement>(
      `[data-inbox-detail] ${actionSelector(selectedItem.kind, action)}`,
    );
    if (!button || button.disabled) return;
    event.preventDefault();
    button.click();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex h-full min-h-0 flex-col overflow-hidden">
  <h1 class="sr-only">Inbox</h1>

  {#if snapshot.isPending}
    <p class="p-4 text-sm text-fg-muted">Loading...</p>
  {:else if snapshot.isError}
    <Alert class="m-2" variant="danger" title="Failed to load snapshot">{snapshot.error.message}</Alert>
  {:else if !snapshot.data.project.initialized}
    <InitializePrompt cwd={snapshot.data.project.cwd} />
  {:else if groups.length === 0}
    {#if runs.isPending}
      <p class="p-4 text-sm text-fg-muted">Loading...</p>
    {:else}
      <InboxZero />
    {/if}
  {:else if showNarrowDetail && selectedItem}
    <div class="flex min-h-0 flex-1 flex-col p-2">
      <WorkbenchPane framed data-inbox-detail-pane class="flex min-h-0 flex-1 flex-col">
        <div class="border-b border-border px-3 py-2">
          <a href={resolve(inboxUrl())} class="inline-flex items-center gap-1.5 text-sm text-fg-muted hover:text-fg">
            <ArrowLeft class="h-4 w-4" aria-hidden="true" />
            <span>Back to inbox</span>
          </a>
        </div>
        <div data-inbox-detail class="flex min-h-0 flex-1 flex-col">
          {#key selectedItem.id}
            <InboxDetail item={selectedItem} {agentAvailable} />
          {/key}
        </div>
      </WorkbenchPane>
    </div>
  {:else}
    <ResizableWorkbenchSplit splitId="inbox" handleLabel="Resize inbox panes">
      {#snippet left()}
        <WorkbenchPane framed class="min-h-0 min-w-0">
          <div class="flex h-full min-h-0 flex-col">
            <PaneHeader title="Inbox" {subtitle}>
              {#snippet actions()}
                {#if agentAvailable}
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    data-inbox-check-conflicts
                    disabled={checkConflicts.isPending || conflictCheckActive}
                    onclick={() => checkConflicts.mutate()}
                  >
                    <ScanSearch class="h-3.5 w-3.5" aria-hidden="true" />
                    <span>{checkConflicts.isPending || conflictCheckActive ? 'Checking…' : 'Check for conflicts'}</span>
                  </Button>
                {/if}
              {/snippet}
            </PaneHeader>

            {#if runs.isError}
              <Alert class="m-2" variant="warning" title="Agent results unavailable">{runs.error.message}</Alert>
            {/if}

            <div data-inbox-list class="min-h-0 flex-1 overflow-auto pb-2">
              {#each groups as group (group.id)}
                <section data-inbox-group={group.id} aria-labelledby={`inbox-group-${group.id}`}>
                  <h3
                    id={`inbox-group-${group.id}`}
                    class="flex items-center gap-1.5 px-3 pb-1 pt-3 text-[11px] font-medium uppercase tracking-wide text-fg-subtle"
                  >
                    <span>{group.label}</span>
                    <span class="font-mono text-fg-subtle/80">{group.items.length}</span>
                  </h3>
                  <ul class="px-1.5">
                    {#each group.items as item (item.id)}
                      {@const badge = kindBadge(item)}
                      {@const active = selectedItem?.id === item.id}
                      <li>
                        <a
                          class="block rounded"
                          href={resolve(inboxUrl(item.id))}
                          data-inbox-item={item.id}
                          aria-current={active ? 'page' : undefined}
                          onclick={(event) => handleRowClick(event, item)}
                        >
                          <ListRow {active} class="py-1.5">
                            <div class="flex items-center gap-2">
                              <Badge variant={badge.variant} mono class="shrink-0 uppercase">{badge.label}</Badge>
                              <span class="min-w-0 flex-1 truncate text-sm font-medium text-fg">{item.title}</span>
                            </div>
                            {#if item.subtitle}
                              <p class="mt-0.5 truncate pl-0.5 text-xs text-fg-muted">{item.subtitle}</p>
                            {/if}
                          </ListRow>
                        </a>
                      </li>
                    {/each}
                  </ul>
                </section>
              {/each}
            </div>
          </div>
        </WorkbenchPane>
      {/snippet}

      {#snippet right()}
        <WorkbenchPane framed data-inbox-detail-pane class="hidden min-h-0 min-w-0 flex-col md:flex">
          <div data-inbox-detail class="flex min-h-0 flex-1 flex-col">
            {#if selectedItem}
              {#key selectedItem.id}
                <InboxDetail item={selectedItem} {agentAvailable} />
              {/key}
            {:else}
              <div class="p-4">
                <EmptyState>Select an item to see what it needs.</EmptyState>
              </div>
            {/if}
          </div>
        </WorkbenchPane>
      {/snippet}
    </ResizableWorkbenchSplit>
  {/if}
</div>
