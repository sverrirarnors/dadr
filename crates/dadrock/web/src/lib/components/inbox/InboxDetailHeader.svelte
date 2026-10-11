<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Badge } from '$lib/components/ui';
  import type { KindBadge } from '$lib/inbox/presentation';

  let {
    badge,
    title,
    children,
  }: {
    badge: KindBadge;
    title: string;
    children?: Snippet;
  } = $props();
</script>

<header data-inbox-detail-header class="min-w-0">
  <div class="flex flex-wrap items-center gap-2">
    <Badge variant={badge.variant} mono class="uppercase">{badge.label}</Badge>
    <h2 class="min-w-0 text-lg font-semibold leading-6 text-fg">{title}</h2>
  </div>
  {#if children}
    <div class="mt-1.5">{@render children()}</div>
  {/if}
</header>
