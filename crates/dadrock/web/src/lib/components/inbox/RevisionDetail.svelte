<script lang="ts">
  import { resolve } from '$app/paths';
  import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { createApi } from '$lib/api';
  import { parseListInput } from '$lib/candidates/form';
  import { Alert, Button, DisclosureRow, SegmentedTabs, TextInput, TextareaInput } from '$lib/components/ui';
  import { ActionBar, MetadataStrip } from '$lib/components/workbench';
  import { diffStats, hasChanges, lineDiff } from '$lib/inbox/diff';
  import type { RevisionItem } from '$lib/inbox/items';
  import { kindBadge, recordLedgerHref } from '$lib/inbox/presentation';
  import { inboxRunsQueryKey } from '$lib/inbox/queries';
  import { stripFrontMatter } from '$lib/markdown/render';
  import { ledgerUrl } from '$lib/navigation/routes';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import { resolvedItems } from '$lib/stores/resolved.svelte';
  import { toasts } from '$lib/stores/toast.svelte';
  import InboxDetailHeader from './InboxDetailHeader.svelte';
  import LineDiff from './LineDiff.svelte';
  import RenderedMarkdown from './RenderedMarkdown.svelte';

  type View = 'changes' | 'edit';

  let { item }: { item: RevisionItem } = $props();

  const api = createApi();
  const queryClient = useQueryClient();

  // The page remounts this component per item, so the form starts from the revision once.
  const initial = (() => item)();
  let view = $state<View>('changes');
  let title = $state(initial.revisedTitle);
  let tagsInput = $state(initial.tags.join(' '));
  let body = $state(initial.body_markdown);

  const tags = $derived(parseListInput(tagsInput));
  const canApply = $derived(title.trim().length > 0 && body.trim().length > 0);
  const recordHref = $derived(item.record ? recordLedgerHref(item.record) : ledgerUrl({ id: item.recordId }));
  const recordTitle = $derived(item.record?.title ?? item.recordId);

  const source = createQuery(() => ({
    queryKey: ['record-source', item.recordId],
    queryFn: () => api.getRecordSource(item.recordId),
  }));

  const currentBody = $derived(source.data ? stripFrontMatter(source.data.content) : '');
  const diff = $derived(lineDiff(currentBody, body));
  const stats = $derived(diffStats(diff));

  const apply = createMutation(() => ({
    mutationFn: () =>
      api.applyRecordRevision({
        record_id: item.recordId,
        title: title.trim(),
        tags,
        body_markdown: body,
      }),
    onSuccess: async (response) => {
      // Read the item before resolving it: the parent re-selects as soon as it disappears.
      const itemId = item.id;
      const revisedTitle = recordTitle;
      resolvedItems.add(itemId);
      await applyMutationResponse(queryClient, response);
      await queryClient.invalidateQueries({ queryKey: inboxRunsQueryKey });
      toasts.push({ severity: 'success', message: response.message, detail: `Revised “${revisedTitle}”.` });
    },
  }));

  function discard() {
    const itemId = item.id;
    toasts.push({ severity: 'info', message: `Discarded the revision of “${recordTitle}”` });
    resolvedItems.add(itemId);
  }
</script>

<div data-inbox-revision class="flex min-h-0 flex-1 flex-col">
  <div class="min-h-0 flex-1 space-y-4 overflow-auto p-4 md:p-5">
    <InboxDetailHeader badge={kindBadge(item)} title={`Revision of “${recordTitle}”`}>
      <MetadataStrip>
        <a class="text-accent hover:underline" href={resolve(recordHref)}>Open the record in the Ledger</a>
        {#if source.data}
          <span aria-hidden="true">·</span>
          <span data-inbox-diff-stats>+{stats.added} −{stats.removed}</span>
        {/if}
      </MetadataStrip>
    </InboxDetailHeader>

    {#if item.change_explanation.trim()}
      <section class="space-y-1">
        <h3 class="text-xs font-medium uppercase tracking-wide text-fg-subtle">What changed and why</h3>
        <p class="text-sm leading-6 text-fg">{item.change_explanation}</p>
      </section>
    {/if}

    {#if item.summary.trim()}
      <DisclosureRow>
        {#snippet summary()}What the agent looked at{/snippet}
        <RenderedMarkdown markdown={item.summary} class="text-fg-muted" />
      </DisclosureRow>
    {/if}

    {#if apply.isError}
      <Alert variant="danger" title="Could not apply the revision">{apply.error.message}</Alert>
    {/if}

    <SegmentedTabs
      ariaLabel="Revision view"
      value={view}
      items={[
        { value: 'changes', label: 'Changes' },
        { value: 'edit', label: 'Edit' },
      ]}
      onselect={(next) => {
        view = next;
      }}
    />

    {#if view === 'changes'}
      {#if source.isPending}
        <p class="text-sm text-fg-muted">Loading the current record…</p>
      {:else if source.isError}
        <Alert variant="warning" title="Current record unavailable">
          {source.error.message}. The diff below treats the revision as entirely new.
        </Alert>
        <LineDiff lines={diff} />
      {:else if !hasChanges(diff)}
        <Alert variant="neutral">The proposed Markdown matches the current record. Applying it will change only the title or tags.</Alert>
      {:else}
        <LineDiff lines={diff} />
      {/if}
    {:else}
      <div class="space-y-3">
        <label class="block space-y-1.5">
          <span class="block text-xs font-medium uppercase tracking-wide text-fg-subtle">Revision title</span>
          <TextInput bind:value={title} />
        </label>
        <label class="block space-y-1.5">
          <span class="block text-xs font-medium uppercase tracking-wide text-fg-subtle">Revision tags</span>
          <TextInput bind:value={tagsInput} placeholder="#dadr/data/processing #dadr/methodology" />
        </label>
        <label class="block space-y-1.5">
          <span class="block text-xs font-medium uppercase tracking-wide text-fg-subtle">Revised record markdown</span>
          <TextareaInput bind:value={body} rows={18} spellcheck="false" class="font-mono text-xs" />
        </label>
      </div>
    {/if}
  </div>

  <ActionBar>
    <Button
      type="button"
      variant="outline"
      size="sm"
      aria-pressed={view === 'edit'}
      class={view === 'edit' ? 'bg-surface-2' : ''}
      data-inbox-action="edit"
      onclick={() => {
        view = view === 'edit' ? 'changes' : 'edit';
      }}
    >
      Edit
    </Button>
    <div class="ml-auto flex flex-wrap items-center gap-2">
      <Button type="button" variant="outline" size="sm" data-inbox-action="secondary" disabled={apply.isPending} onclick={discard}>
        Discard
      </Button>
      <Button type="button" size="sm" data-inbox-action="primary" disabled={!canApply || apply.isPending} onclick={() => apply.mutate()}>
        {apply.isPending ? 'Applying…' : 'Apply'}
      </Button>
    </div>
  </ActionBar>
</div>
