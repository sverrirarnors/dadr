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

<article
  data-workbench-document-pane
  class={cn('min-h-0 overflow-auto bg-bg px-6 py-5 text-sm leading-6 text-fg', className)}
  {...rest}
>
  {@render children?.()}
</article>
