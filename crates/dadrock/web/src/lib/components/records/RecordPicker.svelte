<script lang="ts">
  import type { RecordSummary } from '$lib/api';
  import RecordSelectionDialog from '$lib/components/records/RecordSelectionDialog.svelte';
  import { displayRecordStatus, type RecordStatusFilter } from '$lib/records/filters';

  type Props = {
    records: RecordSummary[];
    selectedIds: string[];
    query: string;
    status: RecordStatusFilter;
    onadd: (recordId: string) => void;
    onremove: (recordId: string) => void;
  };

  let { records, selectedIds, query = $bindable(), status = $bindable(), onadd, onremove }: Props =
    $props();

  let pickerOpen = $state(false);

  function recordById(recordId: string): RecordSummary | undefined {
    return records.find(
      (record) =>
        record.id === recordId ||
        record.simple_record_id === recordId ||
        record.id_prefix === recordId,
    );
  }

  function openPicker() {
    pickerOpen = true;
  }

  function closePicker() {
    pickerOpen = false;
  }

  function addSelectedRecords(recordIds: string[]) {
    for (const recordId of recordIds) {
      onadd(recordId);
    }
    closePicker();
  }

  function addButtonLabel(count: number): string {
    if (count === 0) return 'Add selected';
    return `Add ${count} record${count === 1 ? '' : 's'}`;
  }
</script>

<div data-record-picker class="space-y-3">
  <div data-record-picker-selected>
    {#if selectedIds.length}
      <div class="divide-y divide-border border-y border-border">
        {#each selectedIds as recordId (recordId)}
          {@const record = recordById(recordId)}
          <div class="flex items-start justify-between gap-2 py-2 text-sm">
            <div class="min-w-0">
              <div class="truncate font-medium text-fg">
                {record?.title ?? recordId}
              </div>
              <div class="mt-1 truncate text-xs text-fg-subtle">
                {record ? `${displayRecordStatus(record)} · ${record.path}` : recordId}
              </div>
            </div>
            <button
              type="button"
              class="shrink-0 rounded border border-border bg-surface px-2 py-1 text-xs text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
              aria-label={`Remove ${record?.title ?? recordId}`}
              onclick={() => onremove(recordId)}
            >
              Remove
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <p class="text-sm text-fg-muted">No records selected.</p>
    {/if}
  </div>

  <button
    type="button"
    class="inline-flex h-8 items-center rounded-md border border-border bg-bg px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface hover:text-fg"
    onclick={openPicker}
  >
    Add records
  </button>

  {#if pickerOpen}
    <RecordSelectionDialog
      title="Add records"
      {records}
      {selectedIds}
      bind:query
      bind:status
      confirmLabel={addButtonLabel}
      onconfirm={addSelectedRecords}
      oncancel={closePicker}
    />
  {/if}
</div>
