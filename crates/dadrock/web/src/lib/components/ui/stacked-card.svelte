<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Props = HTMLAttributes<HTMLElement> & {
    class?: string;
    openable?: boolean;
    summary?: Snippet;
    children?: Snippet;
  };

  let { class: className, openable = false, summary, children, ...rest }: Props = $props();
</script>

{#if openable}
  <details data-ui-stacked-card class={cn('rounded-md border border-border bg-bg', className)} {...rest}>
    {#if summary}
      <summary class="cursor-pointer px-4 py-3">
        {@render summary()}
      </summary>
    {/if}
    {@render children?.()}
  </details>
{:else}
  <article data-ui-stacked-card class={cn('rounded-md border border-border bg-bg p-3', className)} {...rest}>
    {@render children?.()}
  </article>
{/if}
