<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { Check, Pencil, Replace, Sparkles, X } from 'lucide-svelte';
  import { untrack } from 'svelte';
  import { createApi, type MutationResponse, type RecordSource } from '$lib/api';
  import { hasMissingRationale } from '$lib/candidates/review';
  import MarkdownEditor from '$lib/components/editor/MarkdownEditor.svelte';
  import { Alert, Badge, Button, InlineLinkBadge } from '$lib/components/ui';
  import { ActionBar, DocumentPane, MetadataStrip } from '$lib/components/workbench';
  import { candidateItemId } from '$lib/inbox/items';
  import { renderMarkdown, stripFrontMatter } from '$lib/markdown/render';
  import { inboxUrl, ledgerRecordUrl, ledgerUrl } from '$lib/navigation/routes';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import { detailActionRecord, detailActions, type DetailSurface } from '$lib/records/detail-state';
  import {
    initialRecordEditorState,
    isRecordEditorDirty,
    syncRecordEditorSource,
  } from '$lib/records/editor-state';
  import { displayRecordStatus, recordStatusVariant } from '$lib/records/filters';
  import { composer } from '$lib/stores/composer.svelte';
  import { toasts } from '$lib/stores/toast.svelte';

  type StatusVariant = 'candidate' | 'accepted' | 'rejected' | 'superseded';

  let {
    id,
    surface,
    embedded = false,
  }: {
    id: string;
    /** The Inbox reviews candidates; the Ledger reads and supersedes decisions. */
    surface: DetailSurface;
    /** True when the panel sits inside a split pane and should scroll internally. */
    embedded?: boolean;
  } = $props();

  const api = createApi();
  const queryClient = useQueryClient();

  const source = createQuery(() => ({
    queryKey: ['record-source', id],
    queryFn: () => api.getRecordSource(id),
    enabled: Boolean(id),
  }));

  const lifecycle = createQuery(() => ({
    queryKey: ['record-lifecycle', id],
    queryFn: () => api.getRecordLifecycle(id),
    enabled: Boolean(id),
  }));

  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  let editorState = $state(initialRecordEditorState());
  let editing = $state(false);

  const dirty = $derived(isRecordEditorDirty(editorState));
  const renderedHtml = $derived(source.data ? renderMarkdown(stripFrontMatter(source.data.content)) : '');

  const record = $derived(detailActionRecord(lifecycle.data?.record, snapshot.data?.records));
  const status = $derived(record ? displayRecordStatus(record) : 'accepted');
  const isCandidate = $derived(status === 'candidate');
  const agentAvailable = $derived(snapshot.data?.settings.agent_available ?? false);
  const actions = $derived(detailActions({ surface, status, agentAvailable }));

  // Links from Inbox issues open the editor with `tab=edit`; the Inbox itself never uses the URL.
  const requestedTab = $derived(surface === 'ledger' ? page.url.searchParams.get('tab') : null);

  $effect(() => {
    const wantsEdit = requestedTab === 'edit';
    void id;
    untrack(() => {
      editing = wantsEdit;
    });
  });

  $effect(() => {
    if (source.data) {
      const nextState = syncRecordEditorSource(editorState, source.data);
      if (nextState !== editorState) {
        editorState = nextState;
      }
    }
  });

  const save = createMutation(() => ({
    mutationFn: (content: string) => api.saveRecordSource(id, content),
    onSuccess: async (response) => {
      const savedContent = editorState.editor;
      queryClient.setQueryData<RecordSource>(['record-source', id], (current) =>
        current ? { ...current, content: savedContent } : current,
      );
      editorState.cleanContent = savedContent;
      editorState.saveMessage = response.message;
      toasts.push({ severity: 'success', message: response.message });
      await applyMutationResponse(queryClient, response);
      leaveEditMode();
    },
  }));

  const accept = createMutation(() => ({
    mutationFn: () => api.acceptCandidate(id),
    onSuccess: (response) => finishCandidateAction(response, 'accepted'),
  }));

  const reject = createMutation(() => ({
    mutationFn: () => api.rejectCandidate(id),
    onSuccess: (response) => finishCandidateAction(response, 'rejected'),
  }));

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  function recordHref(recordId: string) {
    return embedded && surface === 'ledger'
      ? ledgerUrl({ id: recordId, history: true })
      : ledgerRecordUrl(recordId);
  }

  function enterEditMode() {
    editing = true;
  }

  function leaveEditMode() {
    editing = false;
    if (surface !== 'ledger' || !page.url.searchParams.has('tab')) return;
    const next = new URL(page.url);
    next.searchParams.delete('tab');
    void goto(resolve(`${next.pathname}${next.search}` as '/ledger'), {
      replaceState: true,
      noScroll: true,
      keepFocus: true,
    });
  }

  function resetEditor() {
    editorState.editor = editorState.cleanContent;
    editorState.saveMessage = '';
    leaveEditMode();
  }

  function saveEditor() {
    if (!dirty || save.isPending) return;
    editorState.saveMessage = '';
    save.mutate(editorState.editor);
  }

  async function finishCandidateAction(response: MutationResponse, outcome: string) {
    toasts.push({ severity: 'success', message: response.message, detail: `Candidate ${outcome}.` });
    await applyMutationResponse(queryClient, response);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!editing) return;
    if ((event.metaKey || event.ctrlKey) && (event.key.toLowerCase() === 's' || event.code === 'KeyS')) {
      event.preventDefault();
      saveEditor();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if !id}
  <Alert variant="neutral">Select a decision to read it.</Alert>
{:else if source.isPending || lifecycle.isPending}
  <p class="text-sm text-fg-muted">Loading record...</p>
{:else if source.isError || lifecycle.isError}
  <Alert variant="danger" title="Failed to load record">
    {source.error?.message ?? lifecycle.error?.message ?? 'Unknown record loading error'}
  </Alert>
{:else if source.data && lifecycle.data && record}
  <div data-record-detail-header class="flex flex-wrap items-start justify-between gap-4">
    <div class="min-w-0">
      <div class="flex flex-wrap items-center gap-2">
        <Badge variant={recordStatusVariant(record) as StatusVariant} mono class="uppercase">
          {status.slice(0, 3)}
        </Badge>
        <svelte:element
          this={embedded ? 'h2' : 'h1'}
          class={embedded ? 'text-xl font-semibold' : 'text-2xl font-semibold'}
        >
          {record.title}
        </svelte:element>
      </div>
      <div class="mt-2">
        <Badge mono>{fileName(record.path)}</Badge>
        {#if record.tags.length}
          <MetadataStrip class="mt-1">
            {#each record.tags as tag (tag)}
              <InlineLinkBadge href={resolve(ledgerUrl({ tag }))}>{tag}</InlineLinkBadge>
            {/each}
          </MetadataStrip>
        {/if}
      </div>
    </div>

    <div class="flex flex-wrap items-center justify-end gap-2">
      {#if dirty}
        <Badge variant="warning">Unsaved</Badge>
      {:else if editorState.saveMessage}
        <Badge variant="success">Saved</Badge>
      {/if}
    </div>
  </div>

  {#if save.isError}
    <Alert class="mt-4" variant="danger" title="Save failed">{save.error.message}</Alert>
  {:else if accept.isError || reject.isError}
    <Alert class="mt-4" variant="danger" title="Candidate action failed">
      {accept.error?.message ?? reject.error?.message}
    </Alert>
  {/if}

  {#if lifecycle.data.predecessors.length || lifecycle.data.successors.length || lifecycle.data.diagnostics.length}
    <div data-record-supersession class="mt-4 space-y-1 text-xs text-fg-subtle">
      {#if lifecycle.data.predecessors.length}
        <div class="flex flex-wrap items-center gap-x-1.5 gap-y-0.5">
          <span>Supersedes</span>
          {#each lifecycle.data.predecessors as predecessor (predecessor.id)}
            <a class="text-accent hover:underline" href={resolve(recordHref(predecessor.id))}>
              {predecessor.title}
            </a>
          {/each}
        </div>
      {/if}

      {#if lifecycle.data.successors.length}
        <div class="flex flex-wrap items-center gap-x-1.5 gap-y-0.5">
          <span>Superseded by</span>
          {#each lifecycle.data.successors as successor (successor.id)}
            <a class="text-accent hover:underline" href={resolve(recordHref(successor.id))}>
              {successor.title}
            </a>
          {/each}
        </div>
      {/if}

      {#if lifecycle.data.diagnostics.length}
        <ul class="space-y-1">
          {#each lifecycle.data.diagnostics as diagnostic (diagnostic.code + diagnostic.message)}
            <li>
              <span class="font-medium text-warning">{diagnostic.code}</span>
              <span class="ml-2 text-fg-muted">{diagnostic.message}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}

  <ActionBar data-record-actions class="mt-2 border-0 bg-transparent px-0">
    <div class="flex flex-wrap items-center gap-2">
      {#if editing}
        <Button type="button" variant="outline" size="sm" disabled={save.isPending} onclick={resetEditor}>
          Cancel
        </Button>
        <Button type="button" size="sm" disabled={!dirty || save.isPending} onclick={saveEditor}>
          {save.isPending ? 'Saving...' : 'Save'}
        </Button>
      {:else}
        {#each actions.primary as action (action)}
          {#if action === 'edit'}
            <Button type="button" variant="outline" size="sm" data-record-action="edit" onclick={enterEditMode}>
              <Pencil class="h-3.5 w-3.5" aria-hidden="true" />
              <span>Edit</span>
            </Button>
          {:else if action === 'revise'}
            <Button
              type="button"
              variant="outline"
              size="sm"
              data-record-action="revise"
              onclick={() => composer.show({ mode: 'revise', recordIds: [id] })}
            >
              <Sparkles class="h-3.5 w-3.5" aria-hidden="true" />
              <span>Revise with agent</span>
            </Button>
          {/if}
        {/each}
      {/if}
    </div>

    {#if !editing && actions.lifecycle.length}
      <div class="ml-auto flex flex-wrap items-center gap-2">
        {#each actions.lifecycle as action (action)}
          {#if action === 'reject'}
            <Button
              type="button"
              variant="danger"
              size="sm"
              data-record-action="reject"
              disabled={reject.isPending || accept.isPending}
              onclick={() => reject.mutate()}
            >
              <X class="h-3.5 w-3.5" aria-hidden="true" />
              <span>{reject.isPending ? 'Rejecting' : 'Reject'}</span>
            </Button>
          {:else if action === 'accept'}
            <Button
              type="button"
              variant="outline"
              size="sm"
              data-record-action="accept"
              class="border-success/40 bg-success/10 text-success hover:bg-success/20"
              disabled={accept.isPending || reject.isPending || dirty || save.isPending}
              onclick={() => accept.mutate()}
            >
              <Check class="h-3.5 w-3.5" aria-hidden="true" />
              <span>{accept.isPending ? 'Accepting' : 'Accept'}</span>
            </Button>
          {:else if action === 'supersede'}
            <Button
              type="button"
              variant="outline"
              size="sm"
              data-record-action="supersede"
              onclick={() => composer.show({ mode: 'supersede', recordIds: [id] })}
            >
              <Replace class="h-3.5 w-3.5" aria-hidden="true" />
              <span>Supersede</span>
            </Button>
          {/if}
        {/each}
      </div>
    {/if}
  </ActionBar>

  {#if isCandidate}
    {#if surface === 'inbox'}
      <Alert
        class="mt-3"
        variant="neutral"
        title={hasMissingRationale(source.data.content) ? 'Reason for this choice is missing' : 'Review this decision'}
      >
        {#if hasMissingRationale(source.data.content)}
          The draft describes what the code does, but does not document why. Edit the decision to add or correct your reason before accepting it.
        {:else}
          Check both what was done and why. Accepting means you endorse this decision and its explanation.
        {/if}
      </Alert>
      {#if lifecycle.data.pending_supersedes}
        <Alert class="mt-2" variant="neutral" title="Proposed replacement">
          Accepting will replace
          <a class="text-accent underline" href={resolve(recordHref(lifecycle.data.pending_supersedes.id))}>
            {lifecycle.data.pending_supersedes.title}
          </a>
          and preserve its explanation.
        </Alert>
      {/if}
    {:else}
      <Alert class="mt-3" variant="info" data-record-candidate-notice>
        This is a candidate.
        <a class="underline" href={resolve(inboxUrl(candidateItemId(id)))}>Review it in the Inbox.</a>
      </Alert>
    {/if}
  {/if}

  {#if editing}
    <div class={embedded ? 'mt-4 flex min-h-0 flex-1 flex-col' : 'mt-4 block'}>
      <span class="mb-2 block text-sm font-medium text-fg">Raw Markdown</span>
      <MarkdownEditor
        data-record-editor-pane
        bind:value={editorState.editor}
        ariaLabel="Raw Markdown"
        onsave={saveEditor}
        autofocus
        class={embedded ? 'min-h-0 flex-1' : 'h-[28rem] resize-y'}
      />
    </div>
  {:else}
    <DocumentPane
      data-record-rendered-pane
      class={`record-markdown mt-4 rounded-md border border-border bg-bg p-6 ${
        embedded ? 'min-h-0 flex-1 overflow-auto' : ''
      }`}
    >
      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
      {@html renderedHtml}
    </DocumentPane>
  {/if}
{/if}

<style>
  :global(.record-markdown h1) {
    margin-bottom: 0.75rem;
    font-size: 1.5rem;
    font-weight: 650;
  }

  :global(.record-markdown h2) {
    margin-top: 1.5rem;
    margin-bottom: 0.5rem;
    font-size: 1rem;
    font-weight: 650;
  }
</style>
