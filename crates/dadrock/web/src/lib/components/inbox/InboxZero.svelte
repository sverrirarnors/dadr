<script lang="ts">
  import { resolve } from '$app/paths';
  import { PenLine } from 'lucide-svelte';
  import { Button } from '$lib/components/ui';
  import { ledgerUrl } from '$lib/navigation/routes';
  import { composer } from '$lib/stores/composer.svelte';
</script>

<div class="flex min-h-0 flex-1 items-center justify-center overflow-auto p-4">
  <section data-inbox-zero aria-labelledby="inbox-zero-title" class="w-full max-w-md rounded-md border border-border bg-surface p-6 text-center">
    <h2 id="inbox-zero-title" class="text-base font-semibold text-fg">Inbox zero</h2>
    <p class="mt-1 text-sm text-fg-muted">Nothing needs your attention.</p>
    <div class="mt-4 flex flex-wrap items-center justify-center gap-3">
      <Button type="button" size="sm" onclick={() => composer.show({ mode: 'new' })}>
        <PenLine class="h-3.5 w-3.5" aria-hidden="true" />
        <span>Draft a decision</span>
      </Button>
      <a class="text-sm text-accent hover:underline" href={resolve(ledgerUrl())}>Browse the Ledger</a>
    </div>
  </section>
</div>
