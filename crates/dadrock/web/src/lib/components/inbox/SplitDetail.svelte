<script lang="ts">
  import { resolve } from '$app/paths';
  import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { Check, Pencil, X } from 'lucide-svelte';
  import { createApi, type RecordSummary } from '$lib/api';
  import RecordDiffView from '$lib/components/records/RecordDiffView.svelte';
  import { Alert, Button, DisclosureRow } from '$lib/components/ui';
  import { ActionBar, MetadataStrip } from '$lib/components/workbench';
  import type { SplitItem } from '$lib/inbox/items';
  import { kindBadge, recordLedgerHref } from '$lib/inbox/presentation';
  import { ledgerUrl } from '$lib/navigation/routes';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import { supersedesSection } from '$lib/records/supersedes';
  import { toasts } from '$lib/stores/toast.svelte';
  import InboxDetailHeader from './InboxDetailHeader.svelte';
  import RenderedMarkdown from './RenderedMarkdown.svelte';

  /**
   * Candidates that supersede the same decision: the halves of a split, or
   * competing alternatives. The reviewer accepts the ones that belong
   * together; the predecessor gains all of them at once. The rest can be
   * rejected one by one.
   */
  let { item, focusId = null }: { item: SplitItem; focusId?: string | null } = $props();

  const api = createApi();
  const queryClient = useQueryClient();

  let selected = $state<string[]>((() => item.records.map((record) => record.id))());

  const predecessorIds = $derived(item.predecessors.map((predecessor) => predecessor.id));
  const diffs = createQuery(() => ({
    queryKey: ['record-diff', 'split', item.id, item.records.map((record) => record.id).join(',')],
    queryFn: () =>
      Promise.all(
        item.records.map(async (record) => {
          const against = namedPredecessors(record)[0] ?? predecessorIds[0];
          return against ? { id: record.id, result: await api.getRecordDiff(record.id, against) } : null;
        }),
      ),
  }));

  const chosen = $derived(item.records.filter((record) => selected.includes(record.id)));

  const acceptSelected = createMutation(() => ({
    mutationFn: () => {
      const [first, ...rest] = chosen;
      if (!first) throw new Error('Select at least one candidate.');
      return api.acceptCandidate(first.id, rest.map((record) => record.id));
    },
    onSuccess: async (response) => {
      const count = chosen.length;
      toasts.push({
        severity: 'success',
        message: response.message,
        detail: count === 1 ? 'Reject the others, or retarget them at the new decision.' : 'Accepted together.',
      });
      await applyMutationResponse(queryClient, response);
    },
  }));

  const reject = createMutation(() => ({
    mutationFn: (record: RecordSummary) => api.rejectCandidate(record.id),
    onSuccess: async (response, record) => {
      toasts.push({ severity: 'success', message: response.message, detail: `Rejected “${record.title}”.` });
      await applyMutationResponse(queryClient, response);
    },
  }));

  const busy = $derived(acceptSelected.isPending || reject.isPending);

  function namedPredecessors(record: RecordSummary): string[] {
    return (supersedesSection(record.preview)?.links ?? [])
      .map((link) => link.id.toLowerCase())
      .filter((id) => predecessorIds.includes(id));
  }

  function reasonOf(record: RecordSummary): string {
    return supersedesSection(record.preview)?.reason?.trim() ?? '';
  }

  function toggle(id: string, on: boolean) {
    selected = on ? [...selected.filter((value) => value !== id), id] : selected.filter((value) => value !== id);
  }

  function predecessorTitle(id: string | undefined): string {
    return item.predecessors.find((predecessor) => predecessor.id === id)?.title ?? '';
  }
</script>

<div data-inbox-split class="flex min-h-0 flex-1 flex-col">
  <div class="min-h-0 flex-1 space-y-4 overflow-auto p-4 md:p-5">
    <InboxDetailHeader badge={kindBadge(item)} title={item.title}>
      <MetadataStrip>
        <span>Each candidate supersedes</span>
        {#each item.predecessors as predecessor, index (predecessor.id)}
          {index ? ', ' : ''}<a class="text-accent hover:underline" href={resolve(recordLedgerHref(predecessor))}>“{predecessor.title}”</a>
        {/each}
      </MetadataStrip>
    </InboxDetailHeader>

    <Alert variant="neutral" title="Accept the ones that belong together">
      Accept all of them to split the decision, or one of them to choose between alternatives, then reject the rest.
      Accepting moves the superseded decision to the archive unchanged.
    </Alert>

    {#if acceptSelected.isError || reject.isError}
      <Alert variant="danger" title="Could not finish">{acceptSelected.error?.message ?? reject.error?.message}</Alert>
    {/if}

    <ul class="space-y-3">
      {#each item.records as record (record.id)}
        {@const reason = reasonOf(record)}
        {@const entry = diffs.data?.find((candidate) => candidate?.id === record.id)}
        <li
          data-split-candidate={record.id}
          class={`rounded-md border p-3 ${record.id === focusId ? 'border-accent/60' : 'border-border'}`}
        >
          <div class="flex flex-wrap items-start gap-2">
            <label class="flex min-w-0 flex-1 items-start gap-2">
              <input
                type="checkbox"
                class="mt-1 h-3.5 w-3.5 accent-accent"
                aria-label={`Select ${record.title}`}
                checked={selected.includes(record.id)}
                disabled={busy}
                onchange={(event) => toggle(record.id, event.currentTarget.checked)}
              />
              <span class="min-w-0">
                <span class="block text-sm font-semibold text-fg">{record.title}</span>
                <span class="block font-mono text-[11px] text-fg-subtle">{record.id_prefix}</span>
              </span>
            </label>
            <div class="flex items-center gap-1.5">
              <a
                class="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-bg px-2.5 text-sm font-medium text-fg hover:bg-surface"
                href={resolve(ledgerUrl({ id: record.id, archived: true, tab: 'edit' }))}
              >
                <Pencil class="h-3.5 w-3.5" aria-hidden="true" />
                <span>Edit</span>
              </a>
              <Button
                type="button"
                variant="outline"
                size="sm"
                aria-label={`Reject ${record.title}`}
                disabled={busy}
                onclick={() => reject.mutate(record)}
              >
                <X class="h-3.5 w-3.5" aria-hidden="true" />
                <span>Reject</span>
              </Button>
            </div>
          </div>

          <div class="mt-2 pl-5">
            {#if reason}
              <RenderedMarkdown markdown={reason} class="text-fg-muted" />
            {:else}
              <p class="text-xs italic text-fg-subtle">No reason given.</p>
            {/if}
            {#if entry}
              <DisclosureRow class="mt-2" summaryClass="py-1.5 text-xs text-fg-muted" open>
                {#snippet summary()}Changes from “{predecessorTitle(entry.result.old.id) || entry.result.old.title}”{/snippet}
                <RecordDiffView diff={entry.result.diff} />
              </DisclosureRow>
            {:else if diffs.isError}
              <p class="mt-2 text-xs text-warning">Changes unavailable: {diffs.error.message}</p>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  </div>

  <ActionBar>
    <span class="text-xs text-fg-muted">{chosen.length} of {item.records.length} selected</span>
    <div class="ml-auto flex flex-wrap items-center gap-2">
      <Button
        type="button"
        size="sm"
        data-inbox-action="primary"
        disabled={!chosen.length || busy}
        onclick={() => acceptSelected.mutate()}
      >
        <Check class="h-3.5 w-3.5" aria-hidden="true" />
        <span>{acceptSelected.isPending ? 'Accepting…' : 'Accept selected'}</span>
      </Button>
    </div>
  </ActionBar>
</div>
