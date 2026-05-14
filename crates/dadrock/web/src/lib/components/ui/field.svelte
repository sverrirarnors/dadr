<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Props = HTMLAttributes<HTMLDivElement> & {
    label?: string;
    description?: string;
    class?: string;
    children?: Snippet;
  };

  let { label = '', description = '', class: className, children, ...rest }: Props = $props();
</script>

<div data-ui-field class={cn('space-y-1.5', className)} {...rest}>
  {#if label}
    <div class="text-xs font-medium uppercase tracking-wide text-fg-subtle">{label}</div>
  {/if}
  {@render children?.()}
  {#if description}
    <div class="text-xs leading-5 text-fg-subtle">{description}</div>
  {/if}
</div>
