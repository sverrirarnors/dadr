<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { homeHref } from '$lib/navigation/routes';
  import { onMount } from 'svelte';

  onMount(() => {
    goto(resolve(homeHref), { replaceState: true });
  });
</script>

<div class="flex min-h-full items-center justify-center text-sm text-fg-muted">Loading...</div>
