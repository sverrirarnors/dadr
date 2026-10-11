<script lang="ts">
  import { resolve } from '$app/paths';
  import { createMutation, useQueryClient } from '@tanstack/svelte-query';
  import { GitPullRequestArrow } from 'lucide-svelte';
  import { createApi, type RecordSource, type StaleSupersedes } from '$lib/api';
  import { Alert, Button } from '$lib/components/ui';
  import type { LedgerHref } from '$lib/navigation/routes';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import { retargetSource } from '$lib/records/retarget';
  import { toasts } from '$lib/stores/toast.svelte';

  /**
   * A successor candidate whose predecessor was superseded after it was
   * drafted, like a branch whose base moved. Retarget points the link in
   * `## Supersedes` at the decision now in force and saves the candidate.
   */
  let {
    id,
    stale,
    source,
    disabled = false,
    recordHref,
  }: {
    id: string;
    stale: StaleSupersedes;
    /** The candidate's file content. */
    source: string;
    /** True while the candidate has unsaved edits. */
    disabled?: boolean;
    recordHref: (id: string) => LedgerHref;
  } = $props();

  const api = createApi();
  const queryClient = useQueryClient();

  const successors = $derived(stale.current_successors);
  let chosen = $state<string | null>(null);
  const target = $derived(
    successors.find((successor) => successor.id === chosen) ?? (successors.length === 1 ? successors[0] : undefined),
  );

  const retarget = createMutation(() => ({
    mutationFn: async () => {
      if (!target) throw new Error('Choose the decision to point at.');
      const content = retargetSource(source, stale.predecessor.id, [target]);
      const response = await api.saveRecordSource(id, content);
      return { response, content, title: target.title };
    },
    onSuccess: async ({ response, content, title }) => {
      queryClient.setQueryData<RecordSource>(['record-source', id], (current) =>
        current ? { ...current, content } : current,
      );
      toasts.push({ severity: 'success', message: `Now supersedes “${title}”`, detail: response.message });
      await applyMutationResponse(queryClient, response);
    },
  }));
</script>

<Alert variant="warning" title="Predecessor already superseded" data-stale-supersedes={stale.predecessor.id}>
  <p>
    <a class="text-accent underline" href={resolve(recordHref(stale.predecessor.id))}>“{stale.predecessor.title}”</a>
    was superseded after this candidate was drafted. The decision now in force:
    {#each successors as current, index (current.id)}
      {index ? ', ' : ''}<a class="text-accent underline" href={resolve(recordHref(current.id))}>“{current.title}”</a>
    {/each}.
    Retarget this candidate to supersede the current decision, or reject it.
  </p>

  {#if successors.length > 1}
    <fieldset class="mt-2 space-y-1">
      <legend class="text-xs text-fg-muted">Supersede instead</legend>
      {#each successors as successor (successor.id)}
        <label class="flex items-center gap-2 text-sm text-fg">
          <input
            type="radio"
            class="accent-accent"
            name={`retarget-${id}-${stale.predecessor.id}`}
            value={successor.id}
            checked={chosen === successor.id}
            onchange={() => (chosen = successor.id)}
          />
          <span>{successor.title}</span>
        </label>
      {/each}
    </fieldset>
  {/if}

  {#if retarget.isError}
    <p class="mt-2 text-danger">{retarget.error.message}</p>
  {/if}

  <Button
    type="button"
    variant="outline"
    size="sm"
    class="mt-2"
    data-record-action="retarget"
    disabled={disabled || !target || retarget.isPending}
    title={disabled ? 'Save or cancel your edits first' : undefined}
    onclick={() => retarget.mutate()}
  >
    <GitPullRequestArrow class="h-3.5 w-3.5" aria-hidden="true" />
    <span>
      {#if retarget.isPending}
        Retargeting…
      {:else if successors.length === 1 && target}
        Retarget to “{target.title}”
      {:else}
        Retarget
      {/if}
    </span>
  </Button>
</Alert>
