<script lang="ts">
  import { resolve } from '$app/paths';
  import { Sparkles } from 'lucide-svelte';
  import { Badge, Button, KeyValueGrid } from '$lib/components/ui';
  import { buttonVariants } from '$lib/components/ui/button/button.svelte';
  import { ActionBar, MetadataStrip } from '$lib/components/workbench';
  import { diagnosticLocation } from '$lib/diagnostics';
  import { diagnosticFixInstructions, recordEditHref } from '$lib/diagnostics/actions';
  import type { IssueItem } from '$lib/inbox/items';
  import { isCurrentDecision, kindBadge, severityVariant } from '$lib/inbox/presentation';
  import { composer } from '$lib/stores/composer.svelte';
  import { resolvedItems } from '$lib/stores/resolved.svelte';
  import { toasts } from '$lib/stores/toast.svelte';
  import InboxDetailHeader from './InboxDetailHeader.svelte';

  let { item, agentAvailable }: { item: IssueItem; agentAvailable: boolean } = $props();

  const dismissable = $derived(item.diagnostic.severity !== 'error');
  const location = $derived(diagnosticLocation(item.diagnostic));
  const facts = $derived(
    [
      item.diagnostic.path ? { key: 'path', label: 'Path', value: item.diagnostic.path, mono: true } : null,
      item.diagnostic.line ? { key: 'line', label: 'Line', value: String(item.diagnostic.line), mono: true } : null,
      item.diagnostic.field ? { key: 'field', label: 'Field', value: item.diagnostic.field, mono: true } : null,
      item.record ? { key: 'record', label: 'Record', value: item.record.title } : null,
    ].filter((fact): fact is NonNullable<typeof fact> => fact !== null),
  );

  /** Candidates change in place; a decision in force gets a successor; archived records never change. */
  const fixable = $derived(
    item.record ? item.record.status === 'candidate' || isCurrentDecision(item.record) : false,
  );

  function fixWithAgent() {
    if (!item.record) return;
    const current = item.record.status !== 'candidate';
    composer.show({
      mode: current ? 'supersede' : 'revise',
      recordIds: [item.record.id],
      text: diagnosticFixInstructions(item.category, item.diagnostic),
      ...(current ? { view: 'agent' as const } : {}),
    });
  }

  function dismiss() {
    // Read the item before resolving it: the parent re-selects as soon as it disappears.
    const { id, diagnostic } = item;
    toasts.push({ severity: 'info', message: 'Dismissed the issue', detail: diagnostic.message });
    resolvedItems.add(id);
  }
</script>

<div data-inbox-issue class="flex min-h-0 flex-1 flex-col">
  <div class="min-h-0 flex-1 space-y-4 overflow-auto p-4 md:p-5">
    <InboxDetailHeader badge={kindBadge(item)} title={item.diagnostic.message}>
      <MetadataStrip>
        <Badge variant={severityVariant(item.diagnostic.severity)} class="uppercase">{item.diagnostic.severity}</Badge>
        <Badge mono>{item.diagnostic.code}</Badge>
        <span>{item.category}</span>
        {#if location}
          <span aria-hidden="true">·</span>
          <span data-inbox-issue-location>{location}</span>
        {/if}
      </MetadataStrip>
    </InboxDetailHeader>

    {#if facts.length}
      <KeyValueGrid items={facts} />
    {/if}

    {#if item.diagnostic.related_paths.length}
      <section class="space-y-1">
        <h3 class="text-xs font-medium uppercase tracking-wide text-fg-subtle">Related paths</h3>
        <ul class="space-y-0.5 font-mono text-xs text-fg">
          {#each item.diagnostic.related_paths as path (path)}
            <li class="break-all">{path}</li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if !dismissable}
      <p class="text-xs text-fg-subtle">Errors stay in the inbox until the record is fixed; they cannot be dismissed.</p>
    {/if}
  </div>

  <ActionBar>
    {#if item.record}
      <a class={buttonVariants({ variant: 'outline', size: 'sm' })} data-inbox-action="open" href={resolve(recordEditHref(item.record))}>
        Open record
      </a>
      {#if agentAvailable && fixable}
        <Button type="button" variant="outline" size="sm" data-inbox-action="primary" onclick={fixWithAgent}>
          <Sparkles class="h-3.5 w-3.5" aria-hidden="true" />
          <span>Fix with agent</span>
        </Button>
      {/if}
    {/if}
    {#if dismissable}
      <Button type="button" variant="outline" size="sm" class="ml-auto" data-inbox-action="secondary" onclick={dismiss}>
        Dismiss
      </Button>
    {/if}
  </ActionBar>
</div>
