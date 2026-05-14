<script lang="ts" module>
  import { tv, type VariantProps } from 'tailwind-variants';

  export const badgeVariants = tv({
    base: 'inline-flex max-w-full items-center gap-1 rounded px-1.5 py-0.5 text-[11px] font-medium leading-4',
    variants: {
      variant: {
        neutral: 'bg-surface-2 text-fg-muted',
        accent: 'bg-accent-subtle text-accent',
        success: 'bg-success/10 text-success',
        warning: 'bg-warning/10 text-warning',
        danger: 'bg-danger/10 text-danger',
        candidate: 'bg-candidate/20 text-candidate',
        accepted: 'bg-accepted/20 text-accepted',
        rejected: 'bg-rejected/20 text-rejected',
        superseded: 'bg-superseded/20 text-superseded',
      },
      mono: { true: 'font-mono', false: '' },
    },
    defaultVariants: { variant: 'neutral', mono: false },
  });

  export type BadgeVariants = VariantProps<typeof badgeVariants>;
</script>

<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Props = HTMLAttributes<HTMLSpanElement> &
    BadgeVariants & {
      class?: string;
      children?: Snippet;
    };

  let { class: className, variant, mono, children, ...rest }: Props = $props();
</script>

<span data-ui-badge class={cn(badgeVariants({ variant, mono }), className)} {...rest}>
  {@render children?.()}
</span>
