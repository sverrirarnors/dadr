<script lang="ts">
  import { onMount } from 'svelte';
  import { createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { ChevronDown } from 'lucide-svelte';
  import { createApi } from '$lib/api';
  import { Alert, Button } from '$lib/components/ui';
  import { errorMessage } from '$lib/draft/composer-view';
  import { projectFacts, projectName } from '$lib/shell/project-gear';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import { resolvedItems } from '$lib/stores/resolved.svelte';
  import { toasts } from '$lib/stores/toast.svelte';

  const api = createApi();
  const queryClient = useQueryClient();
  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  const name = $derived(projectName(snapshot.data?.project.cwd));
  const facts = $derived(projectFacts(snapshot.data));
  const initialized = $derived(snapshot.data?.project.initialized ?? true);

  let open = $state(false);
  let busy = $state<'initialize' | 'validate' | 'clear' | null>(null);
  let error = $state<string | null>(null);
  let root: HTMLDivElement | undefined;

  onMount(() => {
    const closeOutside = (event: PointerEvent) => {
      if (!root?.contains(event.target as Node)) {
        open = false;
      }
    };
    document.addEventListener('pointerdown', closeOutside);
    return () => document.removeEventListener('pointerdown', closeOutside);
  });

  function handleKeydown(event: KeyboardEvent) {
    if (!open) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      open = false;
    }
  }

  async function run(kind: 'initialize' | 'validate' | 'clear', action: () => Promise<void>) {
    if (busy) return;
    busy = kind;
    error = null;
    try {
      await action();
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = null;
    }
  }

  function initializeProject() {
    return run('initialize', async () => {
      const response = await api.initializeProject();
      toasts.push({ severity: 'success', message: response.message });
      await applyMutationResponse(queryClient, response);
    });
  }

  function validateNow() {
    return run('validate', async () => {
      const response = await api.runValidation('project');
      toasts.push({ severity: 'success', message: response.message });
      await applyMutationResponse(queryClient, response);
    });
  }

  function clearAgentHistory() {
    return run('clear', async () => {
      const response = await api.clearTaskHistory();
      resolvedItems.clear();
      void queryClient.invalidateQueries({ queryKey: ['inbox-runs'] });
      toasts.push({ severity: 'success', message: response.message });
      await applyMutationResponse(queryClient, response);
    });
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div bind:this={root} data-project-gear class="relative inline-flex">
  <button
    type="button"
    class="inline-flex h-8 max-w-48 items-center gap-1 rounded-md px-2 text-sm font-medium text-fg transition-colors hover:bg-surface-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    aria-label="Project"
    aria-haspopup="dialog"
    aria-expanded={open}
    title={snapshot.data?.project.cwd ?? 'Project'}
    onclick={() => (open = !open)}
  >
    <span class="truncate">{name}</span>
    <ChevronDown class="h-3.5 w-3.5 shrink-0 opacity-70" aria-hidden="true" />
  </button>

  {#if open}
    <div
      role="dialog"
      aria-label="Project"
      tabindex="-1"
      class="absolute left-0 top-full z-30 mt-1 w-[min(22rem,calc(100vw-1.5rem))] rounded-md border border-border bg-bg p-3 shadow-lg focus:outline-none"
    >
      {#if snapshot.data}
        <dl class="space-y-2 text-sm">
          {#each facts as fact (fact.key)}
            <div>
              <dt class="text-[11px] uppercase tracking-wide text-fg-subtle">{fact.label}</dt>
              <dd class="mt-0.5 break-all text-fg" class:font-mono={fact.mono} class:text-xs={fact.mono}>
                {fact.value}
              </dd>
            </div>
          {/each}
        </dl>
      {:else if snapshot.isError}
        <Alert variant="danger" title="Could not load the project">{snapshot.error.message}</Alert>
      {:else}
        <p class="text-sm text-fg-subtle">Loading the project…</p>
      {/if}

      {#if error}
        <Alert class="mt-3" variant="danger">{error}</Alert>
      {/if}

      <div class="mt-3 flex flex-wrap gap-2 border-t border-border pt-3">
        {#if !initialized}
          <Button type="button" size="sm" disabled={busy !== null} onclick={initializeProject}>
            {busy === 'initialize' ? 'Initializing' : 'Initialize project'}
          </Button>
        {/if}
        <Button type="button" variant="outline" size="sm" disabled={busy !== null} onclick={validateNow}>
          {busy === 'validate' ? 'Validating' : 'Validate now'}
        </Button>
        <Button type="button" variant="outline" size="sm" disabled={busy !== null} onclick={clearAgentHistory}>
          {busy === 'clear' ? 'Clearing' : 'Clear agent history'}
        </Button>
      </div>

      <p class="mt-3 text-xs text-fg-subtle">Validation runs on every load. Findings land in the Inbox.</p>
    </div>
  {/if}
</div>
