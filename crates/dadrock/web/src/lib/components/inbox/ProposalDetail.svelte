<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { createMutation, useQueryClient } from '@tanstack/svelte-query';
  import { createApi } from '$lib/api';
  import { candidateDestination, parseListInput } from '$lib/candidates/form';
  import MarkdownEditor from '$lib/components/editor/MarkdownEditor.svelte';
  import { Alert, Button, DisclosureRow, InlineLinkBadge, TextInput } from '$lib/components/ui';
  import { ActionBar, DocumentPane, MetadataStrip } from '$lib/components/workbench';
  import type { ProposalItem } from '$lib/inbox/items';
  import { kindBadge, recordLedgerHref } from '$lib/inbox/presentation';
  import { inboxRunsQueryKey } from '$lib/inbox/queries';
  import { ledgerUrl } from '$lib/navigation/routes';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import { resolvedItems } from '$lib/stores/resolved.svelte';
  import { toasts } from '$lib/stores/toast.svelte';
  import InboxDetailHeader from './InboxDetailHeader.svelte';
  import RenderedMarkdown from './RenderedMarkdown.svelte';

  let { item }: { item: ProposalItem } = $props();

  const api = createApi();
  const queryClient = useQueryClient();

  // The page remounts this component per item, so the form starts from the proposal once.
  const initial = (() => item.proposal)();
  let editing = $state(false);
  let title = $state(initial.title);
  let tagsInput = $state(initial.tags.join(' '));
  let body = $state(initial.body_markdown);

  const tags = $derived(parseListInput(tagsInput));
  const canSave = $derived(title.trim().length > 0 && body.trim().length > 0);
  const supersedes = $derived(item.proposal.supersedes ?? []);
  /** Each predecessor, linked in the Ledger; unknown ids link by id. */
  const predecessors = $derived(
    supersedes.map((id) => {
      const record = item.supersededRecords?.find((candidate) => candidate.id === id);
      return { id, title: record?.title ?? id, href: record ? recordLedgerHref(record) : ledgerUrl({ id }) };
    }),
  );

  const save = createMutation(() => ({
    mutationFn: () =>
      api.createCandidate({
        title: title.trim(),
        tags,
        body_markdown: body,
        supersedes: item.proposal.supersedes,
        supersedes_reason: item.proposal.supersedes_reason,
      }),
    onSuccess: async (response) => {
      const itemId = item.id;
      resolvedItems.add(itemId);
      await applyMutationResponse(queryClient, response);
      await queryClient.invalidateQueries({ queryKey: inboxRunsQueryKey });
      toasts.push({ severity: 'success', message: response.message, detail: 'Saved as a candidate for review.' });
      await goto(resolve(candidateDestination(response)));
    },
  }));

  function discard() {
    // Read the item before resolving it: the parent re-selects as soon as it disappears.
    const { id, title: discardedTitle } = item;
    toasts.push({ severity: 'info', message: `Discarded “${discardedTitle}”` });
    resolvedItems.add(id);
  }
</script>

<div data-inbox-proposal class="flex min-h-0 flex-1 flex-col">
  <div class="min-h-0 flex-1 space-y-4 overflow-auto p-4 md:p-5">
    <InboxDetailHeader badge={kindBadge(item)} title={editing ? title || item.title : item.title}>
      <MetadataStrip>
        {#each tags as tag (tag)}
          <InlineLinkBadge href={resolve(ledgerUrl({ tag }))}>{tag}</InlineLinkBadge>
        {/each}
        {#if !supersedes.length}
          <span>{item.subtitle}</span>
        {/if}
      </MetadataStrip>
    </InboxDetailHeader>

    {#if item.proposal.warnings.length}
      <Alert variant="warning" title="The agent flagged">
        <ul class="list-disc pl-4">
          {#each item.proposal.warnings as warning (warning)}
            <li>{warning}</li>
          {/each}
        </ul>
      </Alert>
    {/if}

    {#if predecessors.length}
      <Alert variant="info" title={`Supersedes ${predecessors.map((record) => `“${record.title}”`).join(', ')}`}>
        Accepting the saved candidate supersedes {predecessors.length === 1 ? 'this decision, which moves' : 'these decisions, which move'}
        to the archive unchanged.
        {#each predecessors as predecessor (predecessor.id)}
          <a class="ml-1 text-accent underline" href={resolve(predecessor.href)}>
            Open {predecessors.length === 1 ? 'the predecessor' : `“${predecessor.title}”`}
          </a>
        {/each}
      </Alert>
    {/if}

    {#if item.summary.trim()}
      <DisclosureRow>
        {#snippet summary()}What the agent looked at{/snippet}
        <RenderedMarkdown markdown={item.summary} class="text-fg-muted" />
      </DisclosureRow>
    {/if}

    {#if save.isError}
      <Alert variant="danger" title="Could not save the candidate">{save.error.message}</Alert>
    {/if}

    {#if editing}
      <div class="space-y-3">
        <label class="block space-y-1.5">
          <span class="block text-xs font-medium uppercase tracking-wide text-fg-subtle">Title</span>
          <TextInput bind:value={title} />
        </label>
        <label class="block space-y-1.5">
          <span class="block text-xs font-medium uppercase tracking-wide text-fg-subtle">Tags</span>
          <TextInput bind:value={tagsInput} placeholder="#dadr/data/processing #dadr/methodology" />
        </label>
        <div class="block space-y-1.5">
          <span class="block text-xs font-medium uppercase tracking-wide text-fg-subtle">Candidate Markdown</span>
          <MarkdownEditor bind:value={body} ariaLabel="Candidate Markdown" class="h-[28rem] resize-y" />
        </div>
      </div>
    {:else}
      <DocumentPane class="rounded-md border border-border">
        <RenderedMarkdown markdown={body} />
      </DocumentPane>
    {/if}
  </div>

  <ActionBar>
    <Button
      type="button"
      variant="outline"
      size="sm"
      aria-pressed={editing}
      class={editing ? 'bg-surface-2' : ''}
      data-inbox-action="edit"
      onclick={() => {
        editing = !editing;
      }}
    >
      Edit before saving
    </Button>
    <div class="ml-auto flex flex-wrap items-center gap-2">
      <Button type="button" variant="outline" size="sm" data-inbox-action="secondary" disabled={save.isPending} onclick={discard}>
        Discard
      </Button>
      <Button type="button" size="sm" data-inbox-action="primary" disabled={!canSave || save.isPending} onclick={() => save.mutate()}>
        {save.isPending ? 'Saving…' : 'Save as candidate'}
      </Button>
    </div>
  </ActionBar>
</div>
