<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { onMount } from 'svelte';

  onMount(() => {
    goto(resolve('/inbox'), { replaceState: true });
  });
</script>

<div class="flex min-h-full items-center justify-center text-sm text-fg-muted">Loading...</div>
