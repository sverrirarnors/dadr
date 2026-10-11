<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { useQueryClient } from '@tanstack/svelte-query';
  import { createApi } from '$lib/api';
  import { candidateDestination, parseListInput } from '$lib/candidates/form';
  import MarkdownEditor from '$lib/components/editor/MarkdownEditor.svelte';
  import { Alert, Button, Field, TextInput } from '$lib/components/ui';
  import { errorMessage } from '$lib/draft/composer-view';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import { composer } from '$lib/stores/composer.svelte';
  import { toasts } from '$lib/stores/toast.svelte';

  /** A new decision written by hand. Successors have their own form, DraftSuccessorForm. */
  type Props = {
    agentAvailable: boolean;
    onuseagent: () => void;
  };

  let { agentAvailable, onuseagent }: Props = $props();

  const api = createApi();
  const queryClient = useQueryClient();

  let title = $state('');
  let tags = $state('');
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
      placeholder="#kind/sample #writer/human"
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
