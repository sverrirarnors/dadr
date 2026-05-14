<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Props = HTMLAttributes<HTMLDivElement> & {
    active?: boolean;
    class?: string;
    children?: Snippet;
  };

  let { active = false, class: className, children, ...rest }: Props = $props();
</script>

<div
  data-ui-list-row
  data-active={active ? 'true' : undefined}
  class={cn(
    'rounded px-2 py-1.5 text-sm transition-colors',
    active ? 'bg-accent-subtle text-fg' : 'text-fg hover:bg-surface-2',
    className,
  )}
  {...rest}
>
  {@render children?.()}
</div>
