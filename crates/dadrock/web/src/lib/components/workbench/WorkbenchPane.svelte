<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Props = HTMLAttributes<HTMLElement> & {
    framed?: boolean;
    class?: string;
    children?: Snippet;
  };

  let { framed = false, class: className, children, ...rest }: Props = $props();
</script>

<div
  data-workbench-pane
  class={cn(
    'min-h-0 overflow-hidden',
    framed && 'rounded-md border border-border bg-surface',
    className,
  )}
  {...rest}
>
  {@render children?.()}
</div>
