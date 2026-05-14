<script lang="ts" module>
  import { tv, type VariantProps } from 'tailwind-variants';

  export const buttonVariants = tv({
    base: 'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:pointer-events-none disabled:opacity-50',
    variants: {
      variant: {
        default: 'bg-accent text-accent-fg hover:bg-accent/90',
        outline: 'border border-border bg-bg hover:bg-surface text-fg',
        ghost: 'hover:bg-surface-2 text-fg',
        danger: 'bg-danger text-accent-fg hover:opacity-90',
      },
      size: {
        sm: 'h-8 px-2.5',
        md: 'h-8 px-3',
        lg: 'h-9 px-4',
        icon: 'h-8 w-8',
      },
    },
    defaultVariants: { variant: 'default', size: 'md' },
  });

  export type ButtonVariants = VariantProps<typeof buttonVariants>;
</script>

<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Props = HTMLButtonAttributes &
    ButtonVariants & {
      class?: string;
      children?: Snippet;
    };

  let { class: className, variant, size, children, ...rest }: Props = $props();
</script>

<button data-ui-button class={cn(buttonVariants({ variant, size }), className)} {...rest}>
  {@render children?.()}
</button>
