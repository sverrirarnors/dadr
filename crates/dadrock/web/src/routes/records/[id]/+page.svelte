<script lang="ts">
  import { page } from '$app/state';
  import RecordDetailPanel from '$lib/components/records/RecordDetailPanel.svelte';

  const id = $derived(page.params.id ?? '');
</script>

<div class="h-full overflow-auto p-8">
  <div class="max-w-6xl">
    <RecordDetailPanel {id} />
  </div>
</div>
