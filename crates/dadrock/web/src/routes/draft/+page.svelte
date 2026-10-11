<script lang="ts">
  import DraftComposer from '$lib/components/draft/DraftComposer.svelte';
  import { composer } from '$lib/stores/composer.svelte';

  // A plain link to /draft starts a fresh decision; show() has already
  // prefilled the store when the page is reached from a record or a deep link.
  composer.ensureOpen();
</script>

<DraftComposer />
