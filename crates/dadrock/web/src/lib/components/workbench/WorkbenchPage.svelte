<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Layout = 'single' | 'two-pane' | 'three-pane';
  type Props = HTMLAttributes<HTMLDivElement> & {
    layout?: Layout;
    class?: string;
    element?: HTMLDivElement | null;
    side?: Snippet;
    center?: Snippet;
    inspector?: Snippet;
    children?: Snippet;
  };

  const layouts: Record<Layout, string> = {
    single: 'grid-cols-1',
    'two-pane': 'grid-cols-1 md:grid-cols-[minmax(15rem,22rem)_minmax(0,1fr)]',
    'three-pane': 'grid-cols-[minmax(14rem,20rem)_minmax(0,1fr)_minmax(16rem,22rem)]',
  };

  let {
    layout = 'single',
    class: className,
    element = $bindable(null),
    side,
    center,
    inspector,
    children,
    ...rest
  }: Props = $props();
</script>

<div
  bind:this={element}
  data-workbench-page
  data-layout={layout}
  class={cn('grid min-h-0 flex-1 gap-2 p-2', layouts[layout], className)}
  {...rest}
>
  {#if children}
    {@render children()}
  {:else}
    {#if side}
      {@render side()}
    {/if}
    {#if center}
      {@render center()}
    {/if}
    {#if inspector}
      {@render inspector()}
    {/if}
  {/if}
</div>
