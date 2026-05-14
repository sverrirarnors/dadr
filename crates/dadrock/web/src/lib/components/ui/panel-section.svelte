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

<section
  data-ui-panel-section
  class={cn('rounded-md border border-border bg-surface p-4', className)}
  {...rest}
>
  {@render children?.()}
</section>
