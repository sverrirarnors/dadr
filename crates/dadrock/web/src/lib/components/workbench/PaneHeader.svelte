<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Props = HTMLAttributes<HTMLDivElement> & {
    title?: string;
    subtitle?: string;
    class?: string;
    actions?: Snippet;
    children?: Snippet;
  };

  let { title = '', subtitle = '', class: className, actions, children, ...rest }: Props = $props();
</script>

<div data-workbench-pane-header class={cn('flex min-h-11 items-center justify-between gap-3 border-b border-border px-3 py-2', className)} {...rest}>
  <div class="min-w-0">
    {#if title}
      <h2 class="truncate text-sm font-semibold text-fg">{title}</h2>
    {/if}
    {#if subtitle}
      <p class="truncate text-xs text-fg-subtle">{subtitle}</p>
    {/if}
    {@render children?.()}
  </div>
  {#if actions}
    <div class="flex shrink-0 items-center gap-1.5">{@render actions()}</div>
  {/if}
</div>
