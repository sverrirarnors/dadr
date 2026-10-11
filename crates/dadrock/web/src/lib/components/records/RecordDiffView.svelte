<script lang="ts">
  import type { RecordDiff } from '$lib/api';
  import LineDiff from '$lib/components/inbox/LineDiff.svelte';
  import { Badge } from '$lib/components/ui';
  import { diffStats, fromRecordDiffLines, hasChanges } from '$lib/inbox/diff';
  import { cn } from '$lib/utils';

  /** What changed from one record to the next: title, tags, and the body without `## Supersedes`. */
  let { diff, class: className = '' }: { diff: RecordDiff; class?: string } = $props();

  const lines = $derived(fromRecordDiffLines(diff.lines));
  const stats = $derived(diffStats(lines));
  const bodyChanged = $derived(hasChanges(lines));
</script>

<div data-record-diff class={cn('space-y-2', className)}>
  {#if diff.title || diff.tags_added.length || diff.tags_removed.length}
    <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 text-sm">
      {#if diff.title}
        <dt class="text-xs font-medium uppercase tracking-wide text-fg-subtle">Title</dt>
        <dd data-record-diff-title class="min-w-0">
          <del class="text-fg-muted">{diff.title.old}</del>
          <span aria-hidden="true" class="px-1 text-fg-subtle">→</span>
          <ins class="font-medium text-fg no-underline">{diff.title.new}</ins>
        </dd>
      {/if}
      {#if diff.tags_added.length || diff.tags_removed.length}
        <dt class="text-xs font-medium uppercase tracking-wide text-fg-subtle">Tags</dt>
        <dd data-record-diff-tags class="flex min-w-0 flex-wrap gap-1">
          {#each diff.tags_added as tag (tag)}
            <Badge variant="success" mono>+{tag}</Badge>
          {/each}
          {#each diff.tags_removed as tag (tag)}
            <Badge variant="danger" mono>−{tag}</Badge>
          {/each}
        </dd>
      {/if}
    </dl>
  {/if}

  {#if bodyChanged}
    <p class="text-xs text-fg-subtle" data-record-diff-stats>
      <span class="sr-only">Body: </span>+{stats.added} −{stats.removed} lines
    </p>
    <LineDiff {lines} class="max-h-[32rem]" />
  {:else}
    <p class="text-sm text-fg-muted">The text is unchanged.</p>
  {/if}
</div>
