<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { useQueryClient } from '@tanstack/svelte-query';
  import { untrack } from 'svelte';
  import { createApi, type RecordSummary } from '$lib/api';
  import { candidateDestination, parseListInput } from '$lib/candidates/form';
  import MarkdownEditor from '$lib/components/editor/MarkdownEditor.svelte';
  import { Alert, Button, Field, TextInput } from '$lib/components/ui';
  import { errorMessage, manualPrefill } from '$lib/draft/composer-view';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import { composer, type DraftMode } from '$lib/stores/composer.svelte';
  import { toasts } from '$lib/stores/toast.svelte';

  type Props = {
    mode: DraftMode;
    /** The decision being replaced in supersede mode. */
    record?: RecordSummary | undefined;
    agentAvailable: boolean;
    onuseagent: () => void;
  };

  let { mode, record, agentAvailable, onuseagent }: Props = $props();

  const api = createApi();
  const queryClient = useQueryClient();

  // The form is recreated whenever the composer opens, so the prefill is
  // deliberately read once.
  const prefill = untrack(() => manualPrefill(mode, record));
  let title = $state(prefill.title);
  let tags = $state(prefill.tags);
  let markdown = $state('');
  let templateLoaded = $state(false);
  let creating = $state(false);
  let error = $state<string | null>(null);

  const canCreate = $derived(Boolean(title.trim()) && Boolean(markdown.trim()) && !creating);

  $effect(() => {
    if (templateLoaded) return;
    templateLoaded = true;
    void api
      .getCandidateTemplate()
      .then((template) => {
        if (!markdown.trim()) markdown = template.body;
      })
      .catch(() => {
        // The template is a convenience; the form works without it.
      });
  });

  async function createCandidate() {
    if (!canCreate) return;
    creating = true;
    error = null;
    try {
      const response = await api.createCandidate({
        title: title.trim(),
        tags: parseListInput(tags),
        body_markdown: markdown,
        supersedes: mode === 'supersede' ? record?.id : undefined,
      });
      toasts.push({ severity: 'success', message: response.message });
      await applyMutationResponse(queryClient, response);
      composer.reset();
      await goto(resolve(candidateDestination(response) as '/inbox'));
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      creating = false;
    }
  }
</script>

<form
  data-draft-manual-form
  class="space-y-4"
  onsubmit={(event) => {
    event.preventDefault();
    void createCandidate();
  }}
>
  {#if mode === 'supersede'}
    <Alert variant="info">
      Accepting this replacement will link it to the previous decision and keep the previous explanation as history.
    </Alert>
  {/if}

  <Field label="Title">
    <TextInput
      data-autofocus
      aria-label="Title"
      placeholder="A short, declarative title"
      bind:value={title}
      disabled={creating}
    />
  </Field>

  <Field label="Tags">
    <TextInput
      aria-label="Tags"
      placeholder="#kind/sample #topic/missingness"
      bind:value={tags}
      disabled={creating}
      class="font-mono text-xs"
    />
  </Field>

  <Field label="Candidate Markdown">
    <MarkdownEditor
      ariaLabel="Candidate Markdown"
      class="h-80 resize-y"
      bind:value={markdown}
      readOnly={creating}
    />
  </Field>

  {#if error}
    <Alert variant="danger" title="Could not create the candidate">{error}</Alert>
  {/if}

  <div class="flex flex-wrap items-center gap-3">
    <Button type="submit" disabled={!canCreate}>{creating ? 'Creating' : 'Create candidate'}</Button>
    {#if agentAvailable}
      <button
        type="button"
        class="text-sm text-fg-muted underline-offset-2 hover:text-fg hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        onclick={onuseagent}
      >
        Use the agent instead
      </button>
    {/if}
  </div>
</form>
