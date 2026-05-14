<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Props = HTMLAttributes<HTMLDivElement> & {
    class?: string;
    children?: Snippet;
  };

  let { class: className, children, ...rest }: Props = $props();
</script>

<div data-workbench-pane-toolbar class={cn('flex min-h-10 flex-wrap items-center gap-2 border-b border-border px-3 py-2', className)} {...rest}>
  {@render children?.()}
</div>
