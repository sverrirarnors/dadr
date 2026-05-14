<script lang="ts">
  import { cn } from '$lib/utils';
  import type { HTMLTextareaAttributes } from 'svelte/elements';

  type Props = HTMLTextareaAttributes & {
    class?: string;
  };

  let { class: className, value = $bindable(), ...rest }: Props = $props();
</script>

<textarea
  data-ui-textarea-input
  bind:value
  class={cn(
    'w-full resize-y rounded-md border border-border bg-bg p-3 text-sm leading-6 text-fg placeholder:text-fg-subtle focus:outline-none focus:ring-2 focus:ring-accent disabled:opacity-60',
    className,
  )}
  {...rest}
></textarea>
