<script lang="ts">
  import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { RefreshCw } from 'lucide-svelte';
  import { createApi, type Snapshot } from '$lib/api';
  import { Alert, KeyValueGrid } from '$lib/components/ui';
  import { WorkbenchPage, WorkbenchPane } from '$lib/components/workbench';
  import DiagnosticPanel from '$lib/components/validation/DiagnosticPanel.svelte';
  import { diagnosticActions } from '$lib/diagnostics/actions';
  import { toasts } from '$lib/stores/toast.svelte';

  const api = createApi();
  const queryClient = useQueryClient();

  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  const initialize = createMutation(() => ({
    mutationFn: () => api.initializeProject(),
    onSuccess: async (response) => {
      queryClient.setQueryData<Snapshot>(['snapshot'], response.snapshot);
      toasts.push({ severity: 'success', message: response.message });
      await queryClient.invalidateQueries({ queryKey: ['snapshot'] });
    },
  }));

  const settingsPanel = $derived(
    snapshot.data
      ? {
          title: 'Settings',
          counts: snapshot.data.settings.tag_counts,
          diagnostics: snapshot.data.settings.diagnostics,
        }
      : { title: 'Settings', counts: {}, diagnostics: [] },
  );

  function linesOrFallback(lines: string[], fallback: string): string[] {
    return lines.length ? lines : [fallback];
  }

  function projectItems(data: Snapshot) {
    return [
      { key: 'root', label: 'Root', value: data.project.cwd, mono: true },
      { key: 'accepted', label: 'Accepted', value: data.project.decisions_dir, mono: true },
      { key: 'discovery', label: 'Discovery', value: data.project.discovery_source },
      {
        key: 'state',
        label: 'State',
        value: data.project.initialized ? 'Initialized' : 'Needs initialization',
      },
    ];
  }
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden">
<WorkbenchPage layout="single">
  <WorkbenchPane data-settings-pane class="flex min-h-0 w-full max-w-5xl flex-col overflow-hidden">
    <h1 class="sr-only">Project settings</h1>
    <div data-settings-content class="min-h-0 flex-1 overflow-auto p-4">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <p class="text-sm text-fg-muted">Project discovery, templates, agent state, and initialization.</p>
      {#if snapshot.data && !snapshot.data.project.initialized}
        <button
          type="button"
          class="inline-flex h-9 items-center gap-1.5 rounded-md bg-accent px-3 text-sm font-medium text-accent-fg transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
          disabled={initialize.isPending}
          onclick={() => initialize.mutate()}
        >
          <RefreshCw class={`h-4 w-4 ${initialize.isPending ? 'animate-spin' : ''}`} />
          <span>{initialize.isPending ? 'Initializing' : 'Initialize project'}</span>
        </button>
      {/if}
    </div>

    {#if snapshot.isPending}
      <p class="mt-6 text-sm text-fg-muted">Loading settings...</p>
    {:else if snapshot.isError}
      <Alert class="mt-6" variant="danger" title="Failed to load settings">{snapshot.error.message}</Alert>
    {:else if snapshot.data}
      {#if initialize.isError}
        <Alert class="mt-6" variant="danger" title="Initialization failed">{initialize.error.message}</Alert>
      {/if}

      <section class="mt-4">
        <header>
          <h2 class="text-base font-semibold">Project</h2>
          <p class="mt-1 text-sm text-fg-muted">{snapshot.data.project.status}</p>
        </header>

        <KeyValueGrid class="mt-3" items={projectItems(snapshot.data)} />
      </section>

      <div class="mt-6 grid gap-6 lg:grid-cols-3">
        <section>
          <h2 class="text-base font-semibold">Layout</h2>
          <ul class="mt-3 space-y-2 text-sm text-fg-muted">
            {#each linesOrFallback(snapshot.data.settings.config, 'No project layout entries.') as line (line)}
              <li class="break-words rounded bg-bg px-2 py-1 font-mono text-xs">{line}</li>
            {/each}
          </ul>
        </section>

        <section>
          <h2 class="text-base font-semibold">Templates</h2>
          <ul class="mt-3 space-y-2 text-sm text-fg-muted">
            {#each linesOrFallback(snapshot.data.settings.templates, 'Using built-in templates.') as line (line)}
              <li class="break-words rounded bg-bg px-2 py-1 font-mono text-xs">{line}</li>
            {/each}
          </ul>
        </section>

        <section>
          <h2 class="text-base font-semibold">Agent</h2>
          <ul class="mt-3 space-y-2 text-sm text-fg-muted">
            {#each linesOrFallback(snapshot.data.settings.agent, 'No local agent configured.') as line (line)}
              <li class="break-words rounded bg-bg px-2 py-1 font-mono text-xs">{line}</li>
            {/each}
          </ul>
        </section>
      </div>

      <section class="mt-6">
        <h2 class="text-base font-semibold">Diagnostics</h2>
        <div class="mt-3">
          <DiagnosticPanel
            panel={settingsPanel}
            actionsForDiagnostic={(diagnostic) =>
              diagnosticActions({ category: 'Settings', diagnostic, records: snapshot.data?.records ?? [] })
            }
          />
        </div>
      </section>
    {/if}
    </div>
  </WorkbenchPane>
</WorkbenchPage>
</div>
