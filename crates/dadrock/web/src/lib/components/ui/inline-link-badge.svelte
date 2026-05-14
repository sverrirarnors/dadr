<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAnchorAttributes } from 'svelte/elements';

  type Props = HTMLAnchorAttributes & {
    class?: string;
    mono?: boolean;
    children?: Snippet;
  };

  let { class: className, mono = false, children, ...rest }: Props = $props();
</script>

<a
  data-ui-inline-link-badge
  class={cn(
    'rounded bg-surface-2 px-1.5 py-0.5 text-[11px] text-fg-muted hover:text-fg hover:underline',
    mono && 'font-mono',
    className,
  )}
  {...rest}
>
  {@render children?.()}
</a>
