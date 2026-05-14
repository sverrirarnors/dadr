<script lang="ts">
  import { cn } from '$lib/utils';
  import type { HTMLAttributes } from 'svelte/elements';

  interface KeyValueItem {
    key: string;
    label: string;
    value: string;
    mono?: boolean;
  }

  type Props = HTMLAttributes<HTMLDListElement> & {
    items: KeyValueItem[];
    class?: string;
  };

  let { items, class: className, ...rest }: Props = $props();
</script>

<dl data-ui-key-value-grid class={cn('grid gap-3 text-sm md:grid-cols-2', className)} {...rest}>
  {#each items as item (item.key)}
    <div class="rounded-md border border-border bg-bg p-3">
      <dt class="text-xs uppercase text-fg-subtle">{item.label}</dt>
      <dd class={cn('mt-1 break-all text-fg', item.mono && 'font-mono')}>{item.value}</dd>
    </div>
  {/each}
</dl>
