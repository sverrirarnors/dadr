<script lang="ts" generics="T extends string">
  import { cn } from '$lib/utils';
  import type { HTMLAttributes } from 'svelte/elements';

  interface SegmentedTab<T extends string = string> {
    value: T;
    label: string;
    count?: number | string;
  }

  type Props = Omit<HTMLAttributes<HTMLDivElement>, 'onselect'> & {
    items: SegmentedTab<T>[];
    value: T;
    ariaLabel: string;
    class?: string;
    onselect: (value: T) => void;
  };

  let { items, value, ariaLabel, class: className, onselect, ...rest }: Props = $props();
</script>

<div
  data-ui-segmented-tabs
  role="tablist"
  aria-label={ariaLabel}
  class={cn('flex gap-1 overflow-x-auto', className)}
  {...rest}
>
  {#each items as item (item.value)}
    <button
      type="button"
      role="tab"
      aria-selected={value === item.value}
      class="h-8 shrink-0 rounded px-3 text-sm font-medium text-fg-muted transition-colors hover:bg-surface hover:text-fg"
      class:bg-surface={value === item.value}
      class:text-fg={value === item.value}
      onclick={() => onselect(item.value)}
    >
      {item.label}
      {#if item.count !== undefined}
        <span class="ml-1 text-xs text-fg-subtle">{item.count}</span>
      {/if}
    </button>
  {/each}
</div>
