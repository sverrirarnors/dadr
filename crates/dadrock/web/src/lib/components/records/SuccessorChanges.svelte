<script lang="ts">
  import { resolve } from '$app/paths';
  import { createQuery } from '@tanstack/svelte-query';
  import { createApi, type RecordSummary } from '$lib/api';
  import RenderedMarkdown from '$lib/components/inbox/RenderedMarkdown.svelte';
  import { Alert, SelectInput } from '$lib/components/ui';
  import { diffSides, fromRecordDiffLines } from '$lib/inbox/diff';
  import type { LedgerHref } from '$lib/navigation/routes';
  import { splitSource } from '$lib/records/front-matter';
  import { compareSections } from '$lib/records/sections';
  import { supersedesSection } from '$lib/records/supersedes';
  import RecordDiffView from './RecordDiffView.svelte';
  import SectionCompare from './SectionCompare.svelte';

  /**
   * What a successor candidate changes: the reason from its `## Supersedes`
   * and the diff against its predecessor. A merge opens on the section view
   * and can compare against one predecessor at a time.
   */
  let {
    id,
    predecessors,
    source,
    recordHref,
  }: {
    id: string;
    predecessors: RecordSummary[];
    /** The candidate's file content, for the reason. */
    source: string;
    recordHref: (id: string) => LedgerHref;
  } = $props();

  const api = createApi();

  const predecessorIds = $derived(predecessors.map((predecessor) => predecessor.id));
  const diffs = createQuery(() => ({
    queryKey: ['record-diff', id, predecessorIds.join(',')],
    // Read the ids from the key; see RecordDetailPanel.
    queryFn: ({ queryKey }) => {
      const [, candidate, predecessors] = queryKey.map(String);
      return Promise.all(
        (predecessors ?? '').split(',').map((predecessorId) => api.getRecordDiff(candidate ?? '', predecessorId)),
      );
    },
    enabled: predecessorIds.length > 0,
  }));

  const reason = $derived(supersedesSection(splitSource(source).body)?.reason?.trim() ?? '');
  const merge = $derived(predecessors.length > 1);

  /** 'sections', or the id of the predecessor to diff against. */
  let chosen = $state<string | null>(null);
  const compare = $derived(
    chosen && (chosen === 'sections' || predecessorIds.includes(chosen))
      ? chosen
      : merge
        ? 'sections'
        : (predecessorIds[0] ?? 'sections'),
  );

  const selectedDiff = $derived(diffs.data?.find((result) => result.old.id === compare));
  const comparisons = $derived.by(() => {
    const results = diffs.data ?? [];
    if (!results.length) return [];
    const successorBody = diffSides(fromRecordDiffLines(results[0]?.diff.lines ?? [])).after;
    return compareSections(
      results.map((result) => ({
        id: result.old.id,
        title: result.old.title,
        body: diffSides(fromRecordDiffLines(result.diff.lines)).before,
      })),
      successorBody,
    );
  });
</script>

<section data-successor-changes class="space-y-4">
  <div class="space-y-1">
    <h3 class="text-xs font-medium uppercase tracking-wide text-fg-subtle">Why it changed</h3>
    {#if reason}
      <div data-successor-reason>
        <RenderedMarkdown markdown={reason} />
      </div>
    {:else}
      <Alert variant="warning" data-successor-reason-missing>
        No reason given. Edit the candidate and write why the decision changes under the links in
        <code>## Supersedes</code>.
      </Alert>
    {/if}
  </div>

  <p class="text-sm text-fg-muted">
    Supersedes
    {#each predecessors as predecessor, index (predecessor.id)}
      {index ? ', ' : ''}<a class="text-accent hover:underline" href={resolve(recordHref(predecessor.id))}>“{predecessor.title}”</a>
    {/each}
  </p>

  {#if merge}
    <label class="flex flex-wrap items-center gap-2 text-sm text-fg-muted">
      <span>Compare with</span>
      <SelectInput
        class="w-auto min-w-48"
        aria-label="Compare with"
        value={compare}
        onchange={(event) => (chosen = event.currentTarget.value)}
      >
        <option value="sections">All predecessors, by section</option>
        {#each predecessors as predecessor (predecessor.id)}
          <option value={predecessor.id}>{predecessor.title}</option>
        {/each}
      </SelectInput>
    </label>
  {/if}

  {#if diffs.isPending}
    <p class="text-sm text-fg-muted">Loading the changes…</p>
  {:else if diffs.isError}
    <Alert variant="warning" title="Changes unavailable">{diffs.error.message}</Alert>
  {:else if compare === 'sections'}
    <SectionCompare {comparisons} />
  {:else if selectedDiff}
    <RecordDiffView diff={selectedDiff.diff} />
  {/if}
</section>
