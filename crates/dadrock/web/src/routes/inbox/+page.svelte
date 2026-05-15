<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { candidateInboxHref } from '$lib/navigation/routes';
  import { onMount } from 'svelte';

  onMount(() => {
    goto(resolve(candidateInboxHref), { replaceState: true });
  });
</script>

<div class="p-8 text-sm text-fg-muted">Opening candidate records...</div>
