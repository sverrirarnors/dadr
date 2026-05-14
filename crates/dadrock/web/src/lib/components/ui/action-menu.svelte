<script lang="ts" module>
  export const actionMenuItemClass =
    'flex w-full items-center gap-2 rounded px-2.5 py-2 text-left text-sm text-fg-muted transition-colors hover:bg-surface hover:text-fg focus:bg-surface focus:text-fg focus:outline-none disabled:pointer-events-none disabled:opacity-50';
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import { ChevronDown } from 'lucide-svelte';
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Props = Omit<HTMLButtonAttributes, 'children'> & {
    label: string;
    align?: 'start' | 'end';
    class?: string;
    children?: Snippet;
  };

  let { label, align = 'end', class: className, children, ...rest }: Props = $props();
  let open = $state(false);
  let root: HTMLDivElement | undefined;

  onMount(() => {
    const closeOutside = (event: PointerEvent) => {
      if (!root?.contains(event.target as Node)) {
        open = false;
      }
    };
    document.addEventListener('pointerdown', closeOutside);
    return () => document.removeEventListener('pointerdown', closeOutside);
  });

  function handleKeydown(event: KeyboardEvent) {
    if (!open) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      open = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div bind:this={root} data-action-menu class="relative inline-flex">
  <button
    type="button"
    class={cn(
      'inline-flex h-8 items-center justify-center gap-2 whitespace-nowrap rounded-md border border-border bg-bg px-2.5 text-sm font-medium text-fg transition-colors hover:bg-surface focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:pointer-events-none disabled:opacity-50',
      className,
    )}
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={() => {
      open = !open;
    }}
    {...rest}
  >
    <span>{label}</span>
    <ChevronDown class="h-3.5 w-3.5 opacity-70" aria-hidden="true" />
  </button>

  {#if open}
    <div
      role="menu"
      tabindex="-1"
      class={cn(
        'absolute top-full z-30 mt-1 min-w-56 rounded-md border border-border bg-bg p-1 shadow-lg',
        align === 'end' ? 'right-0' : 'left-0',
      )}
      onkeydown={handleKeydown}
      onclick={(event) => {
        if ((event.target as Element).closest('[role="menuitem"]')) {
          open = false;
        }
      }}
    >
      {@render children?.()}
    </div>
  {/if}
</div>
