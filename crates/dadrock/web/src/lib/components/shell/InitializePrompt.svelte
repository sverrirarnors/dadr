<script lang="ts">
  import { createMutation, useQueryClient } from '@tanstack/svelte-query';
  import { RefreshCw } from 'lucide-svelte';
  import { createApi } from '$lib/api';
  import { Alert, Button } from '$lib/components/ui';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import { toasts } from '$lib/stores/toast.svelte';

  let { cwd }: { cwd: string } = $props();

  const api = createApi();
  const queryClient = useQueryClient();

  const initialize = createMutation(() => ({
    mutationFn: () => api.initializeProject(),
    onSuccess: async (response) => {
      toasts.push({ severity: 'success', message: response.message });
      await applyMutationResponse(queryClient, response);
    },
  }));
</script>

<div class="flex min-h-0 flex-1 items-center justify-center overflow-auto p-4">
  <section
    data-initialize-prompt
    aria-labelledby="initialize-project-title"
    class="w-full max-w-2xl rounded-md border border-border bg-surface p-5 shadow-sm"
  >
    <div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
      <div class="min-w-0">
        <h2 id="initialize-project-title" class="text-base font-semibold text-fg">Initialize Dadrock here?</h2>
        <p class="mt-2 text-sm text-fg-muted">
          No Dadrock project was found in
          <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">{cwd}</code>.
        </p>
      </div>
      <Button
        type="button"
        variant="outline"
        size="lg"
        class="shrink-0"
        disabled={initialize.isPending}
        onclick={() => initialize.mutate()}
      >
        <RefreshCw class={`h-4 w-4 ${initialize.isPending ? 'animate-spin' : ''}`} aria-hidden="true" />
        <span>{initialize.isPending ? 'Initializing' : 'Initialize here'}</span>
      </Button>
    </div>

    <div class="mt-4 space-y-3 text-sm leading-6 text-fg-muted">
      <p>
        This creates the local Dadrock project structure in the current directory:
        <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">dadr/accepted/</code>,
        <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">dadr/candidates/</code>,
        <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">dadr/rejected/</code>,
        <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">dadr/templates/</code>,
        <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">dadr/tasks/</code>,
        and
        <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">dadr/runs/</code>.
      </p>
      <p>
        Choose this only if
        <code class="rounded bg-bg px-1 py-0.5 font-mono text-xs text-fg">{cwd}</code>
        is the project Dadrock should manage.
      </p>
    </div>

    {#if initialize.isError}
      <Alert class="mt-4" variant="danger" title="Initialization failed">{initialize.error.message}</Alert>
    {/if}
  </section>
</div>
