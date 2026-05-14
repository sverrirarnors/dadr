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

<div
  data-ui-empty-state
  class={cn('rounded-md border border-dashed border-border px-4 py-6 text-sm text-fg-subtle', className)}
  {...rest}
>
  {@render children?.()}
</div>
