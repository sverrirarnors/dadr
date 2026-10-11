<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLDetailsAttributes } from 'svelte/elements';

  type Props = HTMLDetailsAttributes & {
    class?: string;
    summaryClass?: string;
    bodyClass?: string;
    summary: Snippet;
    children?: Snippet;
  };

  let { class: className, summaryClass = '', bodyClass = '', summary, children, ...rest }: Props = $props();
</script>

<details data-ui-disclosure-row class={cn('rounded-md border border-border bg-bg', className)} {...rest}>
  <summary class={cn('cursor-pointer px-3 py-2 text-sm text-fg', summaryClass)}>{@render summary()}</summary>
  {#if children}
    <div class={cn('border-t border-border px-3 py-2', bodyClass)}>{@render children()}</div>
  {/if}
</details>
