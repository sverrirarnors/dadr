<script lang="ts">
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { ArrowLeft } from 'lucide-svelte';
  import RecordDetailPanel from '$lib/components/records/RecordDetailPanel.svelte';
  import { ledgerUrl } from '$lib/navigation/routes';

  const id = $derived(page.params.id ?? '');
</script>

<div class="h-full overflow-auto p-6">
  <div class="max-w-6xl">
    <a
      href={resolve(ledgerUrl({ id }))}
      class="inline-flex items-center gap-1.5 text-sm text-fg-muted hover:text-fg"
    >
      <ArrowLeft class="h-4 w-4" aria-hidden="true" />
      <span>Back to ledger</span>
    </a>

    <div class="mt-6">
      <RecordDetailPanel {id} surface="ledger" />
    </div>
  </div>
</div>
