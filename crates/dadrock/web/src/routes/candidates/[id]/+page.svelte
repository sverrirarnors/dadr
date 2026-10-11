<script lang="ts">
  import { hasMissingRationale } from '$lib/candidates/review';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { Check, X } from 'lucide-svelte';
  import { createApi } from '$lib/api';
  import { Alert, Badge, Button, SegmentedTabs } from '$lib/components/ui';
  import { ActionBar, DocumentPane, MetadataStrip, WorkbenchPage, WorkbenchPane } from '$lib/components/workbench';
  import { renderMarkdown, stripFrontMatter } from '$lib/markdown/render';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import { toasts } from '$lib/stores/toast.svelte';

  type ReviewTab = 'rendered' | 'raw';
  const tabs: Array<{ value: ReviewTab; label: string }> = [
    { value: 'rendered', label: 'Rendered' },
    { value: 'raw', label: 'Raw' },
  ];

  const api = createApi();
  const queryClient = useQueryClient();
  const id = $derived(page.params.id ?? '');
  let tab = $state<ReviewTab>('rendered');

  const source = createQuery(() => ({
    queryKey: ['record-source', id],
    queryFn: () => api.getRecordSource(id),
  }));

  const lifecycle = createQuery(() => ({
    queryKey: ['record-lifecycle', id],
    queryFn: () => api.getRecordLifecycle(id),
  }));

  const renderedHtml = $derived(source.data ? renderMarkdown(stripFrontMatter(source.data.content)) : '');
  const isCandidate = $derived(lifecycle.data?.record.status === 'candidate');

  const accept = createMutation(() => ({
    mutationFn: () => api.acceptCandidate(id),
    onSuccess: async (response) => {
      await finishLifecycleAction(response, 'accepted');
      await goto(resolve('/records/[id]', { id: response.focus?.selected_id ?? id }));
    },
  }));

  const reject = createMutation(() => ({
    mutationFn: () => api.rejectCandidate(id),
    onSuccess: async (response) => {
      await finishLifecycleAction(response, 'rejected');
      await goto(resolve('/records?status=candidate'));
    },
  }));

  async function finishLifecycleAction(response: Awaited<ReturnType<typeof api.acceptCandidate>>, severityLabel: string) {
    toasts.push({ severity: 'success', message: response.message, detail: `Candidate ${severityLabel}.` });
    await applyMutationResponse(queryClient, response);
  }
</script>

<WorkbenchPage layout="single">
  <WorkbenchPane class="max-w-6xl overflow-auto">
    <div class="p-6">
    <a href={resolve('/records?status=candidate')} class="text-sm text-fg-muted hover:text-fg">Back to candidates</a>

    {#if source.isPending || lifecycle.isPending}
      <p class="mt-6 text-sm text-fg-muted">Loading candidate...</p>
    {:else if source.isError || lifecycle.isError}
      <Alert class="mt-6" variant="danger" title="Failed to load candidate">
        {source.error?.message ?? lifecycle.error?.message ?? 'Unknown candidate loading error'}
      </Alert>
    {:else if source.data && lifecycle.data}
      <div class="mt-6 flex flex-wrap items-start justify-between gap-4">
        <div class="min-w-0">
          <div class="flex flex-wrap items-center gap-2">
            <Badge variant="candidate" mono class="uppercase">
              candidate
            </Badge>
            <h1 class="text-2xl font-semibold">{lifecycle.data.record.title}</h1>
          </div>
          <MetadataStrip class="mt-2">
            <Badge mono>{source.data.path}</Badge>
            <Badge mono>{source.data.id}</Badge>
          </MetadataStrip>
        </div>

        {#if isCandidate}
          <ActionBar class="rounded-md border border-border">
            <Button
              type="button"
              variant="danger"
              disabled={reject.isPending || accept.isPending}
              onclick={() => reject.mutate()}
            >
              <X class="h-4 w-4" />
              <span>{reject.isPending ? 'Rejecting' : 'Reject candidate'}</span>
            </Button>
            <Button
              type="button"
              variant="outline"
              class="border-success/40 bg-success/10 text-success hover:bg-success/20"
              disabled={accept.isPending || reject.isPending}
              onclick={() => accept.mutate()}
            >
              <Check class="h-4 w-4" />
              <span>{accept.isPending ? 'Accepting' : 'Accept candidate'}</span>
            </Button>
          </ActionBar>
        {/if}
      </div>

      {#each lifecycle.data.diagnostics as diagnostic (diagnostic.code + diagnostic.message)}
        <Alert class="mt-4" variant="danger" title="Decision needs attention">{diagnostic.message}</Alert>
      {/each}

      {#if accept.isError || reject.isError}
        <Alert class="mt-4" variant="danger" title="Candidate action failed">
          {accept.error?.message ?? reject.error?.message}
        </Alert>
      {/if}

      {#if isCandidate}
        <Alert class="mt-4" variant="neutral" title={hasMissingRationale(source.data.content) ? 'Reason for this choice is missing' : 'Review this decision'}>
          Check both what was done and why. Accepting means you endorse the decision and its explanation.
          <a class="text-accent underline" href={resolve(`/records/${id}?tab=edit`)}>Edit the explanation</a>
        </Alert>
        {#if lifecycle.data.pending_supersedes}
          <Alert class="mt-2" variant="neutral" title="Proposed replacement">Accepting will replace {lifecycle.data.pending_supersedes.title} and preserve its explanation.</Alert>
        {/if}
      {/if}

      <SegmentedTabs class="mt-6" items={tabs} value={tab} ariaLabel="Candidate review view" onselect={(value) => (tab = value)} />

      {#if tab === 'rendered'}
        <DocumentPane class="record-markdown mt-4 rounded-md border border-border bg-bg p-6">
          {@html renderedHtml}
        </DocumentPane>
      {:else}
        <pre
          class="mt-4 max-h-[36rem] overflow-auto rounded-md border border-border bg-surface p-4 font-mono text-xs leading-6 text-fg"
        >{source.data.content}</pre>
      {/if}
    {/if}
    </div>
  </WorkbenchPane>
</WorkbenchPage>
