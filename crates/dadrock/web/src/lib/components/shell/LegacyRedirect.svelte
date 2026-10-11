<script lang="ts">
  import { page } from '$app/state';
  import { redirectLegacyUrl } from '$lib/navigation/deep-links';
  import { onMount } from 'svelte';

  onMount(() => {
    redirectLegacyUrl(page.url);
  });
</script>

<div class="p-8 text-sm text-fg-muted">Redirecting...</div>
