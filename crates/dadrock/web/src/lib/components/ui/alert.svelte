<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Variant = 'neutral' | 'info' | 'success' | 'warning' | 'danger';
  const styles: Record<Variant, string> = {
    neutral: 'border-border bg-bg text-fg-muted',
    info: 'border-accent/40 bg-accent-subtle text-accent',
    success: 'border-success/40 bg-success/10 text-success',
    warning: 'border-warning/40 bg-warning/10 text-warning',
    danger: 'border-danger/40 bg-danger/5 text-danger',
  };

  type Props = HTMLAttributes<HTMLDivElement> & {
    variant?: Variant;
    title?: string;
    class?: string;
    children?: Snippet;
  };

  let { variant = 'neutral', title = '', class: className, children, ...rest }: Props = $props();
</script>

<div data-ui-alert class={cn('rounded-md border px-3 py-2 text-sm', styles[variant], className)} {...rest}>
  {#if title}
    <div class="font-medium">{title}</div>
  {/if}
  {#if children}
    <div class={title ? 'mt-1 text-fg-muted' : ''}>{@render children()}</div>
  {/if}
</div>
