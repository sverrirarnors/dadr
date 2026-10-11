<script lang="ts">
  import RecordDetailPanel from '$lib/components/records/RecordDetailPanel.svelte';
  import type { InboxItem } from '$lib/inbox/items';
  import ConflictDetail from './ConflictDetail.svelte';
  import IssueDetail from './IssueDetail.svelte';
  import ProposalDetail from './ProposalDetail.svelte';
  import ReportDetail from './ReportDetail.svelte';
  import RevisionDetail from './RevisionDetail.svelte';
  import RunningDetail from './RunningDetail.svelte';

  let { item, agentAvailable }: { item: InboxItem; agentAvailable: boolean } = $props();
</script>

{#if item.kind === 'candidate'}
  <div data-inbox-candidate class="flex min-h-0 flex-1 flex-col overflow-auto p-4 md:p-5">
    <RecordDetailPanel id={item.record.id} surface="inbox" embedded />
  </div>
{:else if item.kind === 'proposal'}
  <ProposalDetail {item} />
{:else if item.kind === 'revision'}
  <RevisionDetail {item} />
{:else if item.kind === 'conflict'}
  <ConflictDetail {item} />
{:else if item.kind === 'report'}
  <ReportDetail {item} />
{:else if item.kind === 'issue'}
  <IssueDetail {item} {agentAvailable} />
{:else if item.kind === 'running'}
  <RunningDetail {item} />
{/if}
