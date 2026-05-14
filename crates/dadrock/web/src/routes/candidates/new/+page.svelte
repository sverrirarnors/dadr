<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { ArrowLeft } from 'lucide-svelte';
  import { createApi, type MutationResponse, type RecordSummary, type Snapshot } from '$lib/api';
  import { candidateDestination, parseListInput } from '$lib/candidates/form';
  import {
    Alert,
    Button,
    PanelSection,
    SegmentedTabs,
    TextareaInput,
    TextInput,
  } from '$lib/components/ui';
  import { WorkbenchPage, WorkbenchPane } from '$lib/components/workbench';
  import { toasts } from '$lib/stores/toast.svelte';

  const api = createApi();
  const queryClient = useQueryClient();
  const modes: Array<{ value: 'manual' | 'agent'; label: string }> = [
    { value: 'manual', label: 'Manual' },
    { value: 'agent', label: 'Agent draft' },
  ];

  let mode = $state<'manual' | 'agent'>(readMode(page.url.searchParams.get('mode')));
  let title = $state('');
  let tags = $state('');
  let body = $state('');
  let prompt = $state('');
  let promptPacks = $state('');
  let supersessionPrefillApplied = $state(false);
  const supersedesRecordId = $derived(page.url.searchParams.get('supersedes') ?? '');

  const template = createQuery(() => ({
    queryKey: ['candidate-template'],
    queryFn: () => api.getCandidateTemplate(),
  }));

  const supersededRecord = createQuery(() => ({
    queryKey: ['record-lifecycle', supersedesRecordId],
    queryFn: () => api.getRecordLifecycle(supersedesRecordId),
    enabled: Boolean(supersedesRecordId),
  }));

  $effect(() => {
    if (!template.data || body !== '') return;
    if (supersedesRecordId) {
      return;
    }
    body = template.data.body;
  });

  $effect(() => {
    const record = supersededRecord.data?.record;
    if (!template.data || !record || supersessionPrefillApplied) return;
    if (!title.trim()) {
      title = `Supersede ${record.title}`;
    }
    if (!tags.trim()) {
      tags = record.tags.join(' ');
    }
    if (!body.trim()) {
      body = supersessionBody(template.data.body, record);
    }
    supersessionPrefillApplied = true;
  });

  function supersessionBody(templateBody: string, record: RecordSummary): string {
    const note = `This candidate is intended to supersede ${record.title} (${record.id_prefix}).`;
    if (templateBody.includes('## Context')) {
      return templateBody.replace('## Context', `## Context\n\n${note}`);
    }
    return `${note}\n\n${templateBody}`;
  }

  async function finish(response: MutationResponse) {
    queryClient.setQueryData<Snapshot>(['snapshot'], response.snapshot);
    toasts.push({ severity: 'success', message: response.message });
    const destination = candidateDestination(response);
    if (destination.params) {
      await goto(resolve('/records/[id]', destination.params));
    } else {
      await goto(resolve('/records'));
    }
  }

  const manualMutation = createMutation(() => ({
    mutationFn: () =>
      api.createCandidate({
        title: title.trim(),
        tags: parseListInput(tags),
        body_markdown: body,
      }),
    onSuccess: finish,
  }));

  const agentMutation = createMutation(() => ({
    mutationFn: () =>
      api.draftCandidate({
        prompt: prompt.trim(),
        prompt_packs: parseListInput(promptPacks),
        write: true,
      }),
    onSuccess: finish,
  }));

  const busy = $derived(template.isPending || manualMutation.isPending || agentMutation.isPending);
  const manualReady = $derived(title.trim().length > 0 && body.trim().length > 0 && !busy);
  const agentReady = $derived(prompt.trim().length > 0 && !busy);

  function readMode(value: string | null): 'manual' | 'agent' {
    return value === 'agent' ? 'agent' : 'manual';
  }
</script>

<WorkbenchPage layout="single">
  <WorkbenchPane class="max-w-4xl overflow-auto">
    <div class="p-6">
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div>
        <h1 class="text-2xl font-semibold">New Candidate</h1>
        <p class="mt-1 text-sm text-fg-muted">Create a candidate decision manually or from an agent draft.</p>
      </div>
      <a href={resolve('/records')} class="inline-flex items-center gap-1.5 text-sm text-fg-muted hover:text-fg">
        <ArrowLeft class="h-4 w-4" aria-hidden="true" />
        <span>Back to records</span>
      </a>
    </div>

    {#if template.isError}
      <Alert class="mt-6" variant="danger" title="Failed to load candidate template">{template.error.message}</Alert>
    {/if}

    {#if manualMutation.isError || agentMutation.isError}
      <Alert class="mt-6" variant="danger" title="Candidate creation failed">
        {manualMutation.error?.message ?? agentMutation.error?.message}
      </Alert>
    {/if}

    <PanelSection class="mt-6 p-0">
      <div class="border-b border-border p-1">
        <SegmentedTabs items={modes} value={mode} ariaLabel="Candidate creation mode" onselect={(value) => (mode = value)} />
      </div>

      {#if mode === 'manual'}
        <form
          class="space-y-4 p-4"
          onsubmit={(event) => {
            event.preventDefault();
            if (manualReady) manualMutation.mutate();
          }}
        >
          <label class="block">
            <span class="mb-1 block text-sm font-medium text-fg">Title</span>
            <TextInput
              bind:value={title}
              type="text"
              placeholder="Use DuckDB for local joins"
            />
          </label>

          <label class="block">
            <span class="mb-1 block text-sm font-medium text-fg">Tags</span>
            <TextInput
              bind:value={tags}
              type="text"
              placeholder="#dadr/data/processing #kind/model"
            />
          </label>

          <label class="block">
            <span class="mb-1 block text-sm font-medium text-fg">Candidate Markdown</span>
            <TextareaInput
              bind:value={body}
              disabled={template.isPending}
              class="min-h-[24rem] p-4 font-mono text-xs"
              placeholder={template.isPending ? 'Loading candidate template...' : 'Write candidate markdown'}
            />
          </label>

          <div class="flex justify-end">
            <Button
              type="submit"
              disabled={!manualReady}
              size="lg"
            >
              {manualMutation.isPending ? 'Creating...' : 'Create candidate'}
            </Button>
          </div>
        </form>
      {:else}
        <form
          class="space-y-4 p-4"
          onsubmit={(event) => {
            event.preventDefault();
            if (agentReady) agentMutation.mutate();
          }}
        >
          <label class="block">
            <span class="mb-1 block text-sm font-medium text-fg">Draft Prompt</span>
            <TextareaInput
              bind:value={prompt}
              class="min-h-40"
              placeholder="Describe the decision context the agent should turn into a candidate."
            />
          </label>

          <label class="block">
            <span class="mb-1 block text-sm font-medium text-fg">Prompt Packs</span>
            <TextInput
              bind:value={promptPacks}
              type="text"
              placeholder="dadr"
            />
          </label>

          <div class="flex justify-end">
            <Button
              type="submit"
              disabled={!agentReady}
              size="lg"
            >
              {agentMutation.isPending ? 'Drafting...' : 'Draft candidate'}
            </Button>
          </div>
        </form>
      {/if}
    </PanelSection>
    </div>
  </WorkbenchPane>
</WorkbenchPage>
