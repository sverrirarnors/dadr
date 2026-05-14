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

<div data-workbench-metadata-strip class={cn('flex flex-wrap items-center gap-1.5 text-xs text-fg-subtle', className)} {...rest}>
  {@render children?.()}
</div>
