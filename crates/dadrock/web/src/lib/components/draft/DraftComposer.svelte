<script lang="ts">
  // Placeholder: replaced by the Draft composer implementation.
  import { composer } from '$lib/stores/composer.svelte';
</script>

{#if composer.open}
  <div role="dialog" aria-modal="true" aria-label="Draft" class="fixed inset-0 z-40 flex items-start justify-center bg-fg/30 pt-24">
    <div class="w-full max-w-2xl rounded-md border border-border bg-bg p-4">
      <p class="text-sm text-fg-muted">Draft composer ({composer.mode}).</p>
      <button type="button" class="mt-3 text-sm text-accent" onclick={() => composer.hide()}>Close</button>
    </div>
  </div>
{/if}
