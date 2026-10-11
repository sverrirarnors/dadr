<script lang="ts">
  import { resolve } from '$app/paths';
  import type { RecordSummary } from '$lib/api';
  import { Alert, Badge, Button, StackedCard } from '$lib/components/ui';
  import { ActionBar, MetadataStrip } from '$lib/components/workbench';
  import type { ConflictItem } from '$lib/inbox/items';
  import { isCurrentDecision, kindBadge, recordLedgerHref } from '$lib/inbox/presentation';
  import { displayRecordStatus, recordStatusVariant } from '$lib/records/filters';
  import { composer } from '$lib/stores/composer.svelte';
  import { resolvedItems } from '$lib/stores/resolved.svelte';
  import { toasts } from '$lib/stores/toast.svelte';
  import InboxDetailHeader from './InboxDetailHeader.svelte';

  let { item }: { item: ConflictItem } = $props();

  const sides = $derived(
    [
      { label: 'Left', id: item.finding.left_record_id, record: item.left },
      { label: 'Right', id: item.finding.right_record_id, record: item.right },
    ] as Array<{ label: string; id: string; record: RecordSummary | undefined }>,
  );
  const bothCandidates = $derived(item.left?.status === 'candidate' && item.right?.status === 'candidate');
  const replaceable = $derived(
    [item.left, item.right].filter((record): record is RecordSummary => Boolean(record && isCurrentDecision(record))),
  );

  function merge() {
    if (!item.left || !item.right) return;
    composer.show({ mode: 'merge', recordIds: [item.left.id, item.right.id], text: item.finding.conflict_explanation });
  }

  function replace(record: RecordSummary) {
    composer.show({ mode: 'supersede', recordIds: [record.id], text: item.finding.conflict_explanation });
  }

  function dismiss() {
    // Read the item before resolving it: the parent re-selects as soon as it disappears.
    const { id, title: dismissedTitle } = item;
    toasts.push({ severity: 'info', message: `Dismissed “${dismissedTitle}”` });
    resolvedItems.add(id);
  }
</script>

<div data-inbox-conflict class="flex min-h-0 flex-1 flex-col">
  <div class="min-h-0 flex-1 space-y-4 overflow-auto p-4 md:p-5">
    <InboxDetailHeader badge={kindBadge(item)} title={item.title}>
      <MetadataStrip>
        {#if item.finding.relationship}
          <Badge mono>{item.finding.relationship}</Badge>
        {/if}
        <span>Found by the agent</span>
      </MetadataStrip>
    </InboxDetailHeader>

    <div class="grid gap-3 md:grid-cols-2">
      {#each sides as side (side.label)}
        <StackedCard data-inbox-conflict-side={side.label.toLowerCase()} class="flex min-w-0 flex-col gap-2">
          {#if side.record}
            <div class="flex flex-wrap items-center gap-2">
              <Badge variant={recordStatusVariant(side.record)} mono class="uppercase">
                {displayRecordStatus(side.record).slice(0, 3)}
              </Badge>
              <span class="min-w-0 truncate text-sm font-medium text-fg">{side.record.title}</span>
            </div>
            {#if side.record.preview}
              <p class="text-xs leading-5 text-fg-muted">{side.record.preview}</p>
            {/if}
            <a class="mt-auto text-xs text-accent hover:underline" href={resolve(recordLedgerHref(side.record))}>
              Open in the Ledger
            </a>
          {:else}
            <p class="text-sm text-fg-muted">Record not found.</p>
            <p class="break-all font-mono text-xs text-fg-subtle">{side.id || 'no id'}</p>
          {/if}
        </StackedCard>
      {/each}
    </div>

    {#if item.finding.conflict_explanation}
      <section class="space-y-1">
        <h3 class="text-xs font-medium uppercase tracking-wide text-fg-subtle">Why they conflict</h3>
        <p class="text-sm leading-6 text-fg">{item.finding.conflict_explanation}</p>
      </section>
    {/if}

    {#if item.finding.why_might_not_be_conflict}
      <Alert variant="neutral" title="Why this might not be a conflict">{item.finding.why_might_not_be_conflict}</Alert>
    {/if}

    {#if item.finding.suggested_actions.length}
      <section class="space-y-1">
        <h3 class="text-xs font-medium uppercase tracking-wide text-fg-subtle">Suggested actions</h3>
        <ul class="list-disc space-y-0.5 pl-5 text-sm leading-6 text-fg">
          {#each item.finding.suggested_actions as action (action)}
            <li>{action}</li>
          {/each}
        </ul>
      </section>
    {/if}
  </div>

  <ActionBar>
    {#if bothCandidates}
      <Button type="button" size="sm" data-inbox-action="primary" onclick={merge}>Merge</Button>
    {/if}
    {#each replaceable as record, index (record.id)}
      <Button
        type="button"
        variant="outline"
        size="sm"
        data-inbox-action={index === 0 && !bothCandidates ? 'primary' : undefined}
        onclick={() => replace(record)}
      >
        Replace “{record.title}”
      </Button>
    {/each}
    <Button type="button" variant="outline" size="sm" class="ml-auto" data-inbox-action="secondary" onclick={dismiss}>
      Dismiss
    </Button>
  </ActionBar>
</div>
