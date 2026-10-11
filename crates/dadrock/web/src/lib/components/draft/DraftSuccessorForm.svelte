<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { useQueryClient } from '@tanstack/svelte-query';
  import { Paperclip } from 'lucide-svelte';
  import { tick, untrack } from 'svelte';
  import { createApi, type RecordSummary } from '$lib/api';
  import { candidateDestination, parseListInput } from '$lib/candidates/form';
  import MarkdownEditor from '$lib/components/editor/MarkdownEditor.svelte';
  import { Alert, Button, Field, TextareaInput, TextInput } from '$lib/components/ui';
  import {
    errorMessage,
    successorChanged,
    successorPrefill,
    type SuccessorPrefill,
  } from '$lib/draft/composer-view';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import { composer } from '$lib/stores/composer.svelte';
  import { toasts } from '$lib/stores/toast.svelte';

  /**
   * A successor written by hand. It starts from the exact title, tags, and body
   * of the decision it supersedes (or, for a merge, from every predecessor's
   * sections), so the diff in the Inbox shows only what changed. Saving
   * creates a candidate whose `## Supersedes` links the predecessors and
   * carries the reason; the decisions stay in force until it is accepted.
   */
  type Props = {
    /** The decisions the successor supersedes, in order. */
    predecessors: RecordSummary[];
    agentAvailable: boolean;
    onuseagent: () => void;
    onattachrecords: () => void;
  };

  let { predecessors, agentAvailable, onuseagent, onattachrecords }: Props = $props();

  const api = createApi();
  const queryClient = useQueryClient();

  let prefill = $state<SuccessorPrefill | null>(null);
  let loadError = $state<string | null>(null);
  let reason = $state('');
  let title = $state('');
  let tags = $state('');
  let markdown = $state('');
  let creating = $state(false);
  let error = $state<string | null>(null);
  let form = $state<HTMLFormElement | undefined>();

  const merge = $derived(predecessors.length > 1);
  const changed = $derived(
    prefill ? merge || successorChanged(prefill, { title, tags, body: markdown }) : false,
  );
  const missing = $derived(
    !reason.trim()
      ? 'Say why the decision changes.'
      : !title.trim()
        ? 'Give the successor a title.'
        : !markdown.trim()
          ? 'Write the decision.'
          : !changed
            ? 'Nothing has changed yet. Edit the text, then save.'
            : null,
  );
  const canCreate = $derived(Boolean(prefill) && !missing && !creating);

  // The form is recreated whenever the attached decisions change, so it loads once.
  untrack(() => {
    const records = predecessors;
    void Promise.all(records.map((record) => api.getRecordSource(record.id)))
      .then((sources) => {
        const start = successorPrefill(records.map((record, index) => ({ record, source: sources[index]?.content ?? '' })));
        prefill = start;
        title = start.title;
        tags = start.tags;
        markdown = start.body;
        void tick().then(() => form?.querySelector<HTMLElement>('[data-autofocus]')?.focus());
      })
      .catch((cause) => {
        loadError = errorMessage(cause);
      });
  });

  async function createCandidate() {
    if (!canCreate) return;
    creating = true;
    error = null;
    try {
      const ids = predecessors.map((record) => record.id);
      const response = await api.createCandidate({
        title: title.trim(),
        tags: parseListInput(tags),
        body_markdown: markdown,
        supersedes: ids.length === 1 ? ids[0] : ids,
        supersedes_reason: reason.trim(),
      });
      toasts.push({ severity: 'success', message: response.message, detail: 'Review the successor in the Inbox.' });
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
  bind:this={form}
  data-draft-successor-form
  class="space-y-4"
  onsubmit={(event) => {
    event.preventDefault();
    void createCandidate();
  }}
>
  <Alert variant="info">
    Saving drafts a successor candidate in the Inbox.
    {#if merge}
      {predecessors.map((record) => `“${record.title}”`).join(' and ')} stay in force until you accept it.
    {:else if predecessors[0]}
      “{predecessors[0].title}” stays in force until you accept it, then moves to the archive unchanged.
    {/if}
  </Alert>

  <div class="flex flex-wrap items-center gap-1.5 text-xs text-fg-muted" data-draft-predecessors>
    <span>Supersedes</span>
    {#each predecessors as record (record.id)}
      <span class="inline-flex h-6 items-center rounded-md border border-border bg-surface px-2 text-fg">{record.title}</span>
    {/each}
    <button
      type="button"
      class="inline-flex h-6 items-center gap-1 rounded-md px-1.5 text-fg-muted hover:bg-surface-2 hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      onclick={onattachrecords}
    >
      <Paperclip class="h-3 w-3" aria-hidden="true" />
      <span>Add a decision to merge</span>
    </button>
  </div>

  {#if loadError}
    <Alert variant="danger" title="Could not load the decision">{loadError}</Alert>
  {:else if !prefill}
    <p class="text-sm text-fg-subtle">Loading the decision…</p>
  {:else}
    <Field label="Why does this decision change?" description="Required. It is saved in the successor's ## Supersedes section, like a commit message.">
      <TextareaInput
        data-autofocus
        aria-label="Why does this decision change?"
        aria-required="true"
        class="min-h-20"
        placeholder="What did you learn, or what changed in the analysis?"
        bind:value={reason}
        disabled={creating}
      />
    </Field>

    <Field label="Title">
      <TextInput aria-label="Title" placeholder="A short, declarative title" bind:value={title} disabled={creating} />
    </Field>

    <Field label="Tags">
      <TextInput
        aria-label="Tags"
        placeholder="#kind/sample #writer/human"
        bind:value={tags}
        disabled={creating}
        class="font-mono text-xs"
      />
    </Field>

    <Field label="Successor Markdown">
      <MarkdownEditor ariaLabel="Successor Markdown" class="h-96 resize-y" bind:value={markdown} readOnly={creating} />
    </Field>
  {/if}

  {#if error}
    <Alert variant="danger" title="Could not create the candidate">{error}</Alert>
  {/if}

  <div class="flex flex-wrap items-center gap-3">
    <Button type="submit" disabled={!canCreate}>{creating ? 'Creating' : 'Create candidate'}</Button>
    {#if prefill && missing}
      <span data-draft-successor-hint class="text-xs text-fg-subtle">{missing}</span>
    {/if}
    {#if agentAvailable}
      <button
        type="button"
        class="ml-auto text-sm text-fg-muted underline-offset-2 hover:text-fg hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        onclick={onuseagent}
      >
        Use the agent instead
      </button>
    {/if}
  </div>
</form>
