<script lang="ts">
  import type { DiffLine } from '$lib/inbox/diff';
  import { cn } from '$lib/utils';

  let { lines, class: className = '' }: { lines: DiffLine[]; class?: string } = $props();

  const markers = { same: ' ', add: '+', remove: '-' } as const;
  const rowClass = {
    same: 'text-fg-muted',
    add: 'bg-success/15 text-fg',
    remove: 'bg-danger/10 text-fg',
  } as const;
</script>

<div
  data-inbox-diff
  class={cn('overflow-auto rounded-md border border-border bg-bg py-1 font-mono text-xs leading-5', className)}
>
  {#each lines as line, index (index)}
    <div data-diff-line={line.kind} class={cn('flex min-h-5 whitespace-pre px-2', rowClass[line.kind])}>
      <span aria-hidden="true" class="w-4 shrink-0 select-none text-fg-subtle">{markers[line.kind]}</span>
      <span class="sr-only">{line.kind === 'add' ? 'added' : line.kind === 'remove' ? 'removed' : ''}</span>
      <span class="min-w-0 flex-1">{line.text}</span>
    </div>
  {:else}
    <div class="px-2 text-fg-subtle">Nothing to compare yet.</div>
  {/each}
</div>
