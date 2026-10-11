<script lang="ts">
  import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { RefreshCw } from 'lucide-svelte';
  import { createApi } from '$lib/api';
  import { Alert, Button } from '$lib/components/ui';
  import { ActionBar, MetadataStrip } from '$lib/components/workbench';
  import type { RunningItem } from '$lib/inbox/items';
  import { formatStartedAt, kindBadge } from '$lib/inbox/presentation';
  import { inboxRunsQueryKey } from '$lib/inbox/queries';
  import { toasts } from '$lib/stores/toast.svelte';
  import InboxDetailHeader from './InboxDetailHeader.svelte';

  let { item }: { item: RunningItem } = $props();

  const api = createApi();
  const queryClient = useQueryClient();

  const log = createQuery(() => ({
    queryKey: ['inbox-run-log', item.run.id],
    queryFn: () => api.getTaskLog(item.run.id),
    refetchInterval: 1000,
  }));

  const logText = $derived(log.data?.map((entry) => entry.message).join('\n') ?? '');

  let logElement = $state<HTMLPreElement | null>(null);
  $effect(() => {
    if (!logText || !logElement) return;
    logElement.scrollTop = logElement.scrollHeight;
  });

  const cancel = createMutation(() => ({
    mutationFn: () => api.cancelTaskRun(item.run.id),
    onSuccess: async () => {
      // Read the item before refetching: it leaves the list once the run is cancelled.
      const cancelledTitle = item.title;
      await queryClient.invalidateQueries({ queryKey: inboxRunsQueryKey });
      toasts.push({ severity: 'info', message: 'Cancelled the agent run', detail: cancelledTitle });
    },
  }));
</script>

<div data-inbox-running class="flex min-h-0 flex-1 flex-col">
  <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-auto p-4 md:p-5">
    <InboxDetailHeader badge={kindBadge(item)} title={item.title}>
      <MetadataStrip>
        <RefreshCw class="h-3 w-3 animate-spin text-accent" aria-hidden="true" />
        <span>Started {formatStartedAt(item.run.created_at)}</span>
        <span aria-hidden="true">·</span>
        <span class="font-mono">{item.run.task_id}</span>
      </MetadataStrip>
    </InboxDetailHeader>

    <p class="text-sm text-fg-muted">{item.subtitle}</p>

    {#if cancel.isError}
      <Alert variant="danger" title="Could not cancel the run">{cancel.error.message}</Alert>
    {/if}

    <section class="flex min-h-0 flex-1 flex-col gap-1">
      <h3 class="text-xs font-medium uppercase tracking-wide text-fg-subtle">Agent log</h3>
      {#if log.isError}
        <Alert variant="warning" title="Log unavailable">{log.error.message}</Alert>
      {:else}
        <pre
          bind:this={logElement}
          data-inbox-run-log
          aria-live="polite"
          class="min-h-40 flex-1 overflow-auto rounded-md border border-border bg-bg p-3 font-mono text-xs leading-5 text-fg-muted">{logText || 'Waiting for output…'}</pre>
      {/if}
    </section>
  </div>

  <ActionBar>
    <Button type="button" variant="outline" size="sm" class="ml-auto" data-inbox-action="secondary" disabled={cancel.isPending} onclick={() => cancel.mutate()}>
      {cancel.isPending ? 'Cancelling…' : 'Cancel'}
    </Button>
  </ActionBar>
</div>
