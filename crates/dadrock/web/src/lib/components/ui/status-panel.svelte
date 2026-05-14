<script lang="ts">
  import { Hammer, RefreshCw } from 'lucide-svelte';
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Props = HTMLAttributes<HTMLElement> & {
    label: string;
    message: string;
    busy?: boolean;
    open?: boolean;
    disabled?: boolean;
    detailLabel?: string;
    class?: string;
    children?: Snippet;
  };

  let {
    label,
    message,
    busy = false,
    open = $bindable(false),
    disabled = false,
    detailLabel = '',
    class: className,
    children,
    ...rest
  }: Props = $props();
</script>

<section
  data-ui-status-panel
  class={cn('text-sm text-fg-muted', className)}
  aria-label={label}
  {...rest}
>
  <button
    type="button"
    class="flex w-full items-center justify-between gap-3 py-2 text-left"
    aria-expanded={open}
    disabled={disabled}
    onclick={() => {
      if (!disabled) open = !open;
    }}
  >
    <span class="flex min-w-0 items-center gap-2" aria-live="polite">
      {#if busy}
        <RefreshCw class="h-4 w-4 shrink-0 animate-spin text-accent" />
      {:else}
        <Hammer class="h-4 w-4 shrink-0 text-fg-subtle" />
      {/if}
      <span class="min-w-0">{message}</span>
    </span>
    {#if detailLabel}
      <span class="shrink-0 text-xs text-fg-subtle">{detailLabel}</span>
    {/if}
  </button>
  {#if open && children}
    <div class="mt-2">
      {@render children()}
    </div>
  {/if}
</section>
