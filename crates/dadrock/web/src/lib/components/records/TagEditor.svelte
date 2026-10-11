<script lang="ts">
  import { resolve } from '$app/paths';
  import { createMutation, useQueryClient } from '@tanstack/svelte-query';
  import { Tags } from 'lucide-svelte';
  import { tick } from 'svelte';
  import { createApi, type RecordSource } from '$lib/api';
  import { parseListInput } from '$lib/candidates/form';
  import { Button, InlineLinkBadge, TextInput } from '$lib/components/ui';
  import { MetadataStrip } from '$lib/components/workbench';
  import { ledgerUrl } from '$lib/navigation/routes';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import { readFrontMatter, rewriteTags } from '$lib/records/front-matter';
  import { toasts } from '$lib/stores/toast.svelte';

  /**
   * The tags under a record's title. Accepted and rejected records keep their
   * text, but their tags can change in place: saving rewrites only `tags`.
   */
  let {
    id,
    tags,
    source,
    editable,
  }: {
    id: string;
    /** The tags as the project reports them. */
    tags: string[];
    /** The record's file content. */
    source: string;
    editable: boolean;
  } = $props();

  const api = createApi();
  const queryClient = useQueryClient();

  let editing = $state(false);
  let value = $state('');
  let form = $state<HTMLFormElement | undefined>();

  const save = createMutation(() => ({
    mutationFn: async (next: string[]) => {
      const content = rewriteTags(source, next);
      const response = await api.saveRecordSource(id, content);
      return { response, content };
    },
    onSuccess: async ({ response, content }) => {
      queryClient.setQueryData<RecordSource>(['record-source', id], (current) =>
        current ? { ...current, content } : current,
      );
      editing = false;
      toasts.push({ severity: 'success', message: 'Tags saved', detail: response.message });
      await applyMutationResponse(queryClient, response);
    },
  }));

  async function startEditing() {
    // Edit the tags as written in the file, which may differ in spelling from the summary.
    const written = readFrontMatter(source).tags;
    value = (written.length ? written : tags).join(' ');
    save.reset();
    editing = true;
    await tick();
    form?.querySelector('input')?.focus();
  }

  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (save.isPending) return;
    save.mutate(parseListInput(value));
  }
</script>

{#if editing}
  <form bind:this={form} data-tag-editor class="mt-1 flex flex-wrap items-center gap-2" onsubmit={submit}>
    <TextInput
      aria-label="Tags"
      class="h-8 min-w-0 flex-1 font-mono text-xs"
      placeholder="kind/sample topic/attrition"
      bind:value
      disabled={save.isPending}
      onkeydown={(event) => {
        if (event.key === 'Escape') {
          event.preventDefault();
          event.stopPropagation();
          editing = false;
        }
      }}
    />
    <Button type="button" variant="outline" size="sm" disabled={save.isPending} onclick={() => (editing = false)}>
      Cancel
    </Button>
    <Button type="submit" size="sm" disabled={save.isPending}>{save.isPending ? 'Saving…' : 'Save tags'}</Button>
    {#if save.isError}
      <p class="w-full text-xs text-danger">{save.error.message}</p>
    {/if}
  </form>
{:else if tags.length || editable}
  <MetadataStrip class="mt-1">
    {#each tags as tag (tag)}
      <InlineLinkBadge href={resolve(ledgerUrl({ tag }))}>{tag}</InlineLinkBadge>
    {/each}
    {#if editable}
      <button
        type="button"
        data-record-action="edit-tags"
        class="inline-flex h-5 items-center gap-1 rounded px-1 text-[11px] text-fg-subtle hover:bg-surface-2 hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        aria-label="Edit tags"
        title="Edit tags"
        onclick={startEditing}
      >
        <Tags class="h-3 w-3" aria-hidden="true" />
        {#if !tags.length}<span>Add tags</span>{/if}
      </button>
    {/if}
  </MetadataStrip>
{/if}
