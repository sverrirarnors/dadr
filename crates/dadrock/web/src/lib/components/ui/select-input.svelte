<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLSelectAttributes } from 'svelte/elements';

  type Props = HTMLSelectAttributes & {
    class?: string;
    children?: Snippet;
  };

  let { class: className, value = $bindable(), children, ...rest }: Props = $props();
</script>

<select
  data-ui-select-input
  bind:value
  class={cn(
    'h-9 w-full rounded-md border border-border bg-bg px-3 text-sm text-fg focus:outline-none focus:ring-2 focus:ring-accent disabled:opacity-60',
    className,
  )}
  {...rest}
>
  {@render children?.()}
</select>
