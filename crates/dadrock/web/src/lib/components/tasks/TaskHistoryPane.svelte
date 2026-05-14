<script lang="ts">
  import { RefreshCw } from 'lucide-svelte';
  import type { TaskRun } from '$lib/api';
  import { Alert, Badge, Button, EmptyState, ListRow } from '$lib/components/ui';

  type Props = {
    runs: TaskRun[];
    selectedRunId: string;
    busy: boolean;
    loading: boolean;
    errorMessage: string;
    refreshing: boolean;
    clearing: boolean;
    onRefresh: () => void;
    onClear: () => void;
    onLoadRun: (runId: string) => void;
  };

  let {
    runs,
    selectedRunId,
    busy,
    loading,
    errorMessage,
    refreshing,
    clearing,
    onRefresh,
    onClear,
    onLoadRun,
  }: Props = $props();

  function shortDate(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return date.toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  }
</script>

<section data-task-history-pane class="h-full bg-surface">
  <header class="flex items-start justify-between gap-3 border-b border-border p-4">
    <div>
      <h2 class="text-lg font-semibold">Result history</h2>
      <p class="mt-1 text-sm text-fg-muted">{runs.length} result{runs.length === 1 ? '' : 's'}</p>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <Button type="button" variant="outline" size="sm" disabled={busy} onclick={onRefresh}>
        <RefreshCw class={`h-3.5 w-3.5 ${refreshing ? 'animate-spin' : ''}`} />
        <span>Refresh</span>
      </Button>
      <Button type="button" variant="outline" size="sm" disabled={busy || runs.length === 0} onclick={onClear}>
        {clearing ? 'Clearing' : 'Clear history'}
      </Button>
    </div>
  </header>
  <div class="p-4">
    {#if loading}
      <p class="text-sm text-fg-muted">Loading results...</p>
    {:else if errorMessage}
      <Alert variant="danger" title="Failed to load task results">{errorMessage}</Alert>
    {:else if runs.length}
      <div class="space-y-2">
        {#each runs as run (run.id)}
          <button
            type="button"
            class="block w-full text-left"
            onclick={() => onLoadRun(run.id)}
          >
            <ListRow active={run.id === selectedRunId} class="grid gap-2 p-3 md:grid-cols-[1fr_auto]">
              <span>
                <span class="block font-medium text-fg">{run.task_id}</span>
              </span>
              <span class="flex flex-wrap items-center gap-2 text-xs text-fg-muted">
                <Badge>{run.status}</Badge>
                <span>{shortDate(run.created_at)}</span>
              </span>
            </ListRow>
          </button>
        {/each}
      </div>
    {:else}
      <EmptyState>No task results yet.</EmptyState>
    {/if}
  </div>
</section>
