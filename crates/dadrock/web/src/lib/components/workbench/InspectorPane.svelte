<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Props = HTMLAttributes<HTMLElement> & {
    class?: string;
    children?: Snippet;
  };

  let { class: className, children, ...rest }: Props = $props();
</script>

<aside
  data-workbench-inspector-pane
  class={cn('min-h-0 overflow-auto rounded-md border border-border bg-surface px-3 py-3 text-sm', className)}
  {...rest}
>
  {@render children?.()}
</aside>
