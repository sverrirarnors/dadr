<script lang="ts">
  import { Button } from '$lib/components/ui';
  import { ActionBar, DocumentPane, MetadataStrip } from '$lib/components/workbench';
  import type { ReportItem } from '$lib/inbox/items';
  import { kindBadge } from '$lib/inbox/presentation';
  import { resolvedItems } from '$lib/stores/resolved.svelte';
  import { toasts } from '$lib/stores/toast.svelte';
  import InboxDetailHeader from './InboxDetailHeader.svelte';
  import RenderedMarkdown from './RenderedMarkdown.svelte';

  let { item }: { item: ReportItem } = $props();

  function dismiss() {
    // Read the item before resolving it: the parent re-selects as soon as it disappears.
    const { id, title: dismissedTitle } = item;
    toasts.push({ severity: 'info', message: `Dismissed “${dismissedTitle}”` });
    resolvedItems.add(id);
  }
</script>

<div data-inbox-report class="flex min-h-0 flex-1 flex-col">
  <div class="min-h-0 flex-1 space-y-4 overflow-auto p-4 md:p-5">
    <InboxDetailHeader badge={kindBadge(item)} title={item.title}>
      <MetadataStrip>
        <span>{item.subtitle}</span>
        <span aria-hidden="true">·</span>
        <span class="font-mono">{item.taskId}</span>
      </MetadataStrip>
    </InboxDetailHeader>

    {#if item.summary.trim() && item.summary.trim() !== item.body_markdown.trim()}
      <p class="text-sm leading-6 text-fg-muted">{item.summary}</p>
    {/if}

    <DocumentPane class="rounded-md border border-border">
      <RenderedMarkdown markdown={item.body_markdown} />
    </DocumentPane>
  </div>

  <ActionBar>
    <Button type="button" variant="outline" size="sm" class="ml-auto" data-inbox-action="secondary" onclick={dismiss}>
      Dismiss
    </Button>
  </ActionBar>
</div>
