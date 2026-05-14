<script lang="ts">
  import { cn } from '$lib/utils';
  import type { HTMLInputAttributes } from 'svelte/elements';

  type Props = HTMLInputAttributes & {
    class?: string;
  };

  let { class: className, type = 'text', value = $bindable(), ...rest }: Props = $props();
</script>

<input
  data-ui-text-input
  {type}
  bind:value
  class={cn(
    'h-9 w-full rounded-md border border-border bg-bg px-3 text-sm text-fg placeholder:text-fg-subtle focus:outline-none focus:ring-2 focus:ring-accent disabled:opacity-60',
    className,
  )}
  {...rest}
/>
