<script lang="ts">
  import { hasMissingRationale } from '$lib/candidates/review';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { ArrowLeft, Check, FilePlus2, Sparkles, X } from 'lucide-svelte';
  import { createApi, type MutationResponse, type RecordSource, type RecordSummary } from '$lib/api';
  import { ActionMenu, actionMenuItemClass, Alert, Badge, Button, InlineLinkBadge } from '$lib/components/ui';
  import RecordSelectionDialog from '$lib/components/records/RecordSelectionDialog.svelte';
  import { ActionBar, DocumentPane, MetadataStrip } from '$lib/components/workbench';
  import { renderMarkdown, stripFrontMatter } from '$lib/markdown/render';
  import { detailActionRecord } from '$lib/records/detail-state';
  import {
    displayRecordStatus,
    recordStatusVariant,
    type RecordStatusFilter,
  } from '$lib/records/filters';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import {
    initialRecordEditorState,
    isRecordEditorDirty,
    syncRecordEditorSource,
  } from '$lib/records/editor-state';
  import { toasts } from '$lib/stores/toast.svelte';

  type DetailTab = 'rendered' | 'edit';
  type DetailTarget =
    | '/records'
    | `/records?${string}`
    | `/records/${string}`
    | `/records/${string}?tab=${DetailTab}`;

  type StatusVariant = 'candidate' | 'accepted' | 'rejected' | 'superseded';

  let { id, showBackLink = true, embedded = false } = $props<{
    id: string;
    showBackLink?: boolean;
    embedded?: boolean;
  }>();

  const api = createApi();
  const queryClient = useQueryClient();
  const selectedTab = $derived(readTab(page.url.searchParams.get('tab')));

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

  let editorState = $state(initialRecordEditorState());
  let supersedePickerOpen = $state(false);
  let supersedeSearchQuery = $state('');
  let supersedeStatusFilter = $state<RecordStatusFilter>('accepted');

  const dirty = $derived(isRecordEditorDirty(editorState));
  const renderedHtml = $derived(source.data ? renderMarkdown(stripFrontMatter(source.data.content)) : '');

  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  const detailRecord = $derived(detailActionRecord(lifecycle.data?.record, snapshot.data?.records));
  const isCandidate = $derived(detailRecord?.status === 'candidate');
  const canSupersede = $derived(detailRecord ? displayRecordStatus(detailRecord) === 'accepted' : false);

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
      setTab('rendered');
    },
  }));

  const accept = createMutation(() => ({
    mutationFn: () => api.acceptCandidate(id),
    onSuccess: async (response) => {
      await finishCandidateAction(response, 'accepted');
      const selectedId = response.focus?.selected_id ?? id;
      await goto(resolve(embedded ? `/records?id=${selectedId}` : `/records/${selectedId}`));
    },
  }));

  const reject = createMutation(() => ({
    mutationFn: () => api.rejectCandidate(id),
    onSuccess: async (response) => {
      await finishCandidateAction(response, 'rejected');
      await goto(resolve('/records?status=candidate'));
    },
  }));

  const supersedeExisting = createMutation(() => ({
    mutationFn: (to: string[]) => api.supersedeRecords({ from: id, to }),
    onSuccess: async (response) => {
      supersedePickerOpen = false;
      toasts.push({ severity: 'success', message: response.message });
      await applyMutationResponse(queryClient, response);
      if (embedded) {
        await goto(resolve(`/records?status=superseded&id=${encodeURIComponent(id)}`));
      }
    },
  }));

  function readTab(value: string | null): DetailTab {
    return value === 'edit' ? 'edit' : 'rendered';
  }

  function setTab(tab: DetailTab) {
    const target = embedded ? embeddedTarget(tab) : detailTarget(tab);
    goto(resolve(target), {
      replaceState: true,
      noScroll: true,
      keepFocus: true,
    });
  }

  function embeddedTarget(tab: DetailTab): DetailTarget {
    const next = new URL(page.url);
    next.searchParams.set('id', id);
    if (tab === 'rendered') {
      next.searchParams.delete('tab');
    } else {
      next.searchParams.set('tab', tab);
    }
    const target = (next.search ? `/records${next.search}` : '/records') as
      | '/records'
      | `/records?${string}`;
    return target;
  }

  function detailTarget(tab: DetailTab): DetailTarget {
    const base = `/records/${encodeURIComponent(id)}`;
    return (tab === 'rendered' ? base : `${base}?tab=${tab}`) as DetailTarget;
  }

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  function recordLink(record: RecordSummary): DetailTarget {
    if (embedded) {
      return `/records?id=${record.id}`;
    }
    return `/records/${record.id}`;
  }

  function taskLink(taskId: string, recordId: string): '/tasks' | `/tasks?${string}` {
    return `/tasks?task=${encodeURIComponent(taskId)}&record=${encodeURIComponent(recordId)}`;
  }

  function tagSearchLink(tag: string): DetailTarget {
    return `/records?scope=tag&q=${encodeURIComponent(tag)}`;
  }

  function supersedeConfirmLabel(): string {
    return 'Save';
  }

  function resetEditor() {
    editorState.editor = editorState.cleanContent;
    editorState.saveMessage = '';
    setTab('rendered');
  }

  function saveEditor() {
    if (!dirty || save.isPending) return;
    editorState.saveMessage = '';
    save.mutate(editorState.editor);
  }

  async function finishCandidateAction(response: MutationResponse, severityLabel: string) {
    toasts.push({ severity: 'success', message: response.message, detail: `Candidate ${severityLabel}.` });
    await applyMutationResponse(queryClient, response);
  }

  function handleKeydown(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && (event.key.toLowerCase() === 's' || event.code === 'KeyS')) {
      event.preventDefault();
      saveEditor();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if showBackLink}
  <a href={resolve('/records')} class="inline-flex items-center gap-1.5 text-sm text-fg-muted hover:text-fg">
    <ArrowLeft class="h-4 w-4" aria-hidden="true" />
    <span>Back to records</span>
  </a>
{/if}

{#if !id}
  <Alert variant="neutral">Select a record to read its content.</Alert>
{:else if source.isPending || lifecycle.isPending}
  <p class={showBackLink ? 'mt-6 text-sm text-fg-muted' : 'text-sm text-fg-muted'}>Loading record...</p>
{:else if source.isError || lifecycle.isError}
  <Alert class={showBackLink ? 'mt-6' : ''} variant="danger" title="Failed to load record">
    {source.error?.message ?? lifecycle.error?.message ?? 'Unknown record loading error'}
  </Alert>
{:else if source.data && lifecycle.data}
  <div class={showBackLink ? 'mt-6 flex flex-wrap items-start justify-between gap-4' : 'flex flex-wrap items-start justify-between gap-4'}>
    <div class="min-w-0">
      <div class="flex flex-wrap items-center gap-2">
        <Badge variant={recordStatusVariant(detailRecord ?? lifecycle.data.record) as StatusVariant} mono class="uppercase">
          {displayRecordStatus(detailRecord ?? lifecycle.data.record).slice(0, 3)}
        </Badge>
        <h1 class={embedded ? 'text-xl font-semibold' : 'text-2xl font-semibold'}>{(detailRecord ?? lifecycle.data.record).title}</h1>
      </div>
      <div class="mt-2">
        <Badge mono>{fileName((detailRecord ?? lifecycle.data.record).path)}</Badge>
        {#if (detailRecord ?? lifecycle.data.record).tags.length}
          <MetadataStrip class="mt-1">
            {#each (detailRecord ?? lifecycle.data.record).tags as tag (tag)}
              <InlineLinkBadge href={resolve(tagSearchLink(tag))}>
                {tag}
              </InlineLinkBadge>
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
  {:else if supersedeExisting.isError}
    <Alert class="mt-4" variant="danger" title="Supersede failed">
      {supersedeExisting.error.message}
    </Alert>
  {/if}

  {#if lifecycle.data.predecessors.length || lifecycle.data.successors.length || lifecycle.data.diagnostics.length}
    <div class="mt-4 space-y-1 text-xs text-fg-subtle">
      {#if lifecycle.data.predecessors.length}
        <div class="flex flex-wrap items-center gap-1.5">
          <span>Supersedes</span>
          {#each lifecycle.data.predecessors as record (record.id)}
            <a class="font-mono text-accent hover:underline" href={resolve(recordLink(record))}>
              {record.id_prefix}
            </a>
          {/each}
        </div>
      {/if}

      {#if lifecycle.data.successors.length}
        <div class="flex flex-wrap items-center gap-1.5">
          <span>Superseded by</span>
          {#each lifecycle.data.successors as record (record.id)}
            <a class="font-mono text-accent hover:underline" href={resolve(recordLink(record))}>
              {record.id_prefix}
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

  <ActionBar class="mt-2 border-0 bg-transparent px-0">
    <div class="flex flex-wrap items-center gap-2">
      {#if selectedTab === 'edit'}
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={save.isPending}
          onclick={resetEditor}
        >
          Cancel
        </Button>
        <Button
          type="button"
          size="sm"
          disabled={!dirty || save.isPending}
          onclick={saveEditor}
        >
          {save.isPending ? 'Saving...' : 'Save'}
        </Button>
      {:else if isCandidate}
        <ActionMenu label="Edit" align="start">
          <button
            role="menuitem"
            type="button"
            class={actionMenuItemClass}
            onclick={() => setTab('edit')}
          >
            <span>Edit</span>
          </button>
          <a
            role="menuitem"
            class={actionMenuItemClass}
            href={resolve(taskLink('edit-record', lifecycle.data.record.id))}
          >
            <Sparkles class="h-3.5 w-3.5" aria-hidden="true" />
            <span>Edit with agent</span>
          </a>
        </ActionMenu>
      {:else}
        <ActionMenu label="Edit" align="start">
          <button
            role="menuitem"
            type="button"
            class={actionMenuItemClass}
            onclick={() => setTab('edit')}
          >
            <span>Edit</span>
          </button>
          <a
            role="menuitem"
            class={actionMenuItemClass}
            href={resolve(taskLink('edit-record', lifecycle.data.record.id))}
          >
            <Sparkles class="h-3.5 w-3.5" aria-hidden="true" />
            <span>Edit with agent</span>
          </a>
        </ActionMenu>
      {/if}
    </div>

    <div class="ml-auto flex flex-wrap items-center gap-2">
      {#if isCandidate}
        <Button
          type="button"
          variant="danger"
          size="sm"
          disabled={reject.isPending || accept.isPending}
          onclick={() => reject.mutate()}
        >
          <X class="h-3.5 w-3.5" />
          <span>{reject.isPending ? 'Rejecting' : 'Reject'}</span>
        </Button>
        <Button
          type="button"
          variant="outline"
          size="sm"
          class="border-success/40 bg-success/10 text-success hover:bg-success/20"
          disabled={accept.isPending || reject.isPending || dirty || save.isPending}
          onclick={() => accept.mutate()}
        >
          <Check class="h-3.5 w-3.5" />
          <span>{accept.isPending ? 'Accepting' : 'Accept'}</span>
        </Button>
      {:else if canSupersede}
        <ActionMenu label="Change this decision">
          <button
            role="menuitem"
            type="button"
            class={actionMenuItemClass}
            disabled={supersedeExisting.isPending || snapshot.isPending}
            onclick={() => {
              supersedePickerOpen = true;
              supersedeStatusFilter = 'accepted';
            }}
          >
            <span>{supersedeExisting.isPending ? 'Superseding' : 'Replace with existing decisions'}</span>
          </button>
          <a
            role="menuitem"
            class={actionMenuItemClass}
            href={resolve(`/candidates/new?supersedes=${encodeURIComponent((detailRecord ?? lifecycle.data.record).id)}`)}
          >
            <FilePlus2 class="h-3.5 w-3.5" aria-hidden="true" />
            <span>Draft a replacement</span>
          </a>
          <a
            role="menuitem"
            class={actionMenuItemClass}
            href={resolve(taskLink('supersede-record', (detailRecord ?? lifecycle.data.record).id))}
          >
            <Sparkles class="h-3.5 w-3.5" aria-hidden="true" />
            <span>Draft a replacement with agent</span>
          </a>
        </ActionMenu>
      {/if}
    </div>
  </ActionBar>

  {#if isCandidate}
    <Alert class="mt-3" variant="neutral" title={hasMissingRationale(source.data.content) ? 'Reason for this choice is missing' : 'Review this decision'}>
      {#if hasMissingRationale(source.data.content)}
        The draft describes what the code does, but does not document why. Edit the decision to add or correct your reason before accepting it.
      {:else}
        Check both what was done and why. Accepting means you endorse this decision and its explanation.
      {/if}
    </Alert>
    {#if lifecycle.data.pending_supersedes}
      <Alert class="mt-2" variant="neutral" title="Proposed replacement">
        Accepting will replace <a class="text-accent underline" href={resolve(recordLink(lifecycle.data.pending_supersedes))}>{lifecycle.data.pending_supersedes.title}</a> and preserve its explanation.
      </Alert>
    {/if}
  {/if}

  {#if supersedePickerOpen && snapshot.data}
    <RecordSelectionDialog
      title="Replace with existing decisions"
      records={snapshot.data.records}
      selectedIds={[]}
      excludedIds={[lifecycle.data.record.id]}
      bind:query={supersedeSearchQuery}
      bind:status={supersedeStatusFilter}
      confirmLabel={supersedeConfirmLabel}
      onconfirm={(recordIds) => supersedeExisting.mutate(recordIds)}
      oncancel={() => {
        supersedePickerOpen = false;
      }}
    />
  {/if}

  {#if selectedTab === 'rendered'}
    <DocumentPane
      data-record-rendered-pane
      class={`record-markdown mt-4 rounded-md border border-border bg-bg p-6 ${
        embedded ? 'min-h-0 flex-1 overflow-auto' : ''
      }`}
    >
      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
      {@html renderedHtml}
    </DocumentPane>
  {:else}
    <label class={embedded ? 'mt-4 flex min-h-0 flex-1 flex-col' : 'mt-4 block'}>
      <span class="mb-2 block text-sm font-medium text-fg">Raw Markdown</span>
      <textarea
        data-record-editor-pane
        bind:value={editorState.editor}
        spellcheck="false"
        class={`w-full rounded-md border border-border bg-surface p-4 font-mono text-xs leading-6 text-fg focus:outline-none focus:ring-2 focus:ring-accent ${
          embedded ? 'min-h-0 flex-1 resize-none overflow-auto' : 'min-h-[28rem] resize-y'
        }`}
        aria-label="Raw Markdown"
      ></textarea>
    </label>
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
