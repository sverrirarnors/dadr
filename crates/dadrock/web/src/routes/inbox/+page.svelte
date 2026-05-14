<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { onMount } from 'svelte';

  onMount(() => {
    goto(resolve('/records?status=candidate'), { replaceState: true });
  });
</script>

<div class="p-8 text-sm text-fg-muted">Opening candidate records...</div>
