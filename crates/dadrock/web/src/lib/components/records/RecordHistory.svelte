<script lang="ts">
  import { resolve } from '$app/paths';
  import { GitFork, GitMerge, Pencil } from 'lucide-svelte';
  import type { RecordHistory } from '$lib/api';
  import RenderedMarkdown from '$lib/components/inbox/RenderedMarkdown.svelte';
  import { Badge, DisclosureRow } from '$lib/components/ui';
  import { diffStats, fromRecordDiffLines } from '$lib/inbox/diff';
  import { editCountLabel, editDay, editScope, versionEdits } from '$lib/history/edits';
  import { historyLayout } from '$lib/history/layout';
  import type { LedgerHref } from '$lib/navigation/routes';
  import RecordDiffView from './RecordDiffView.svelte';

  /**
   * The versions of one decision, newest first, drawn as lanes: a split joins
   * at the shared predecessor, a merge forks from the successor. Each link
   * shows its reason and, on request, what changed. Each version lists the
   * text edits Git recorded since it was accepted or rejected. Records on the
   * other side of a split or merge are one-line entries.
   */
  let { history, recordHref }: { history: RecordHistory; recordHref: (id: string) => LedgerHref } = $props();

  const LANE = 16;
  const DOT_Y = 12;

  const layout = $derived(historyLayout(history));
  const x = (lane: number) => lane * LANE + LANE / 2;
</script>

<section data-record-history aria-label="History">
  <ol class="space-y-0">
    {#each layout.rows as row (row.version.id)}
      {@const center = x(row.lane)}
      <li
        data-history-version={row.version.id}
        data-history-opened={row.opened ? 'true' : undefined}
        data-history-current={row.version.current ? 'true' : undefined}
        class="flex"
      >
        <div aria-hidden="true" class="relative shrink-0 self-stretch" style:width={`${layout.lanes * LANE}px`}>
          {#each row.through as lane (lane)}
            <span class="absolute bottom-0 top-0 w-px bg-fg-subtle/50" style:left={`${x(lane)}px`}></span>
          {/each}
          {#if row.fromAbove}
            <span class="absolute top-0 w-px bg-fg-subtle/50" style:left={`${center}px`} style:height={`${DOT_Y}px`}></span>
          {/if}
          {#if row.toBelow}
            <span class="absolute bottom-0 w-px bg-fg-subtle/50" style:left={`${center}px`} style:top={`${DOT_Y}px`}></span>
          {/if}
          {#each row.joins as lane (lane)}
            <span class="absolute top-0 w-px bg-fg-subtle/50" style:left={`${x(lane)}px`} style:height={`${DOT_Y}px`}></span>
            <span
              class="absolute h-px bg-fg-subtle/50"
              style:top={`${DOT_Y}px`}
              style:left={`${Math.min(x(lane), center)}px`}
              style:width={`${Math.abs(x(lane) - center)}px`}
            ></span>
          {/each}
          {#each row.forks as lane (lane)}
            <span
              class="absolute h-px bg-fg-subtle/50"
              style:top={`${DOT_Y}px`}
              style:left={`${Math.min(x(lane), center)}px`}
              style:width={`${Math.abs(x(lane) - center)}px`}
            ></span>
            <span class="absolute bottom-0 w-px bg-fg-subtle/50" style:left={`${x(lane)}px`} style:top={`${DOT_Y}px`}></span>
          {/each}
          <span
            class={`absolute h-2.5 w-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 ${
              row.opened
                ? 'border-accent bg-accent'
                : row.version.current
                  ? 'border-success bg-bg'
                  : 'border-fg-subtle bg-bg'
            }`}
            style:left={`${center + 0.5}px`}
            style:top={`${DOT_Y}px`}
          ></span>
        </div>

        <div class="min-w-0 flex-1 pb-4 pl-1">
          <div
            class={`flex min-h-6 flex-wrap items-center gap-x-2 gap-y-0.5 rounded px-1.5 ${
              row.opened ? 'bg-accent-subtle' : ''
            }`}
          >
            <span class="font-mono text-[11px] text-fg-subtle">{row.version.short_id}</span>
            {#if row.opened}
              <span class="text-sm font-semibold text-fg" aria-current="true">{row.version.title}</span>
            {:else}
              <a class="text-sm font-medium text-accent hover:underline" href={resolve(recordHref(row.version.id))}>
                {row.version.title}
              </a>
            {/if}
            {#if row.version.current}
              <Badge variant="success">current</Badge>
            {/if}
            {#if row.version.status === 'rejected'}
              <Badge variant="rejected">rejected</Badge>
            {/if}
          </div>

          {#if versionEdits(row.version).length}
            {@const edits = versionEdits(row.version)}
            <div class="mt-1 px-1.5">
              <DisclosureRow data-history-edits={row.version.id} summaryClass="py-1.5 text-xs text-fg-muted">
                {#snippet summary()}
                  <Pencil class="mr-1 inline h-3 w-3 align-[-2px]" aria-hidden="true" />{editCountLabel(
                    edits.length,
                    row.version.status,
                  )}
                {/snippet}
                <ol class="space-y-1.5">
                  {#each edits as edit, index (edit.commit ?? `working-tree-${index}`)}
                    {@const day = editDay(edit)}
                    {@const stats = edit.diff ? diffStats(fromRecordDiffLines(edit.diff.lines)) : null}
                    <li data-history-edit={edit.short_commit ?? 'working-tree'}>
                      {#snippet editSummary()}
                        {#if edit.commit}
                          {#if day && edit.date}<time datetime={edit.date} title={edit.date}>{day}</time>{/if}
                          {#if edit.author}<span class="text-fg">{edit.author}</span>{/if}
                          {#if edit.subject}<span class="text-fg-muted">{edit.subject}</span>{/if}
                          <span class="font-mono text-fg-subtle">{edit.short_commit}</span>
                        {:else}
                          <span class="font-medium text-warning">Not committed yet</span>
                        {/if}
                        <span class="text-fg-subtle">{editScope(edit)}</span>
                        {#if stats}
                          <span class="font-mono text-fg-subtle">+{stats.added} −{stats.removed}</span>
                        {/if}
                      {/snippet}
                      {#if edit.diff}
                        <DisclosureRow summaryClass="py-1.5 text-xs text-fg-muted [&>*]:mr-2">
                          {#snippet summary()}{@render editSummary()}{/snippet}
                          <RecordDiffView diff={edit.diff} />
                        </DisclosureRow>
                      {:else}
                        <p class="px-3 py-1.5 text-xs text-fg-muted [&>*]:mr-2">{@render editSummary()}</p>
                      {/if}
                    </li>
                  {/each}
                </ol>
              </DisclosureRow>
            </div>
          {/if}

          {#if row.supersedes.length || row.mergedWith.length}
            <div class="mt-1 space-y-1.5 px-1.5">
              {#if row.reason}
                <div data-history-reason>
                  <RenderedMarkdown markdown={row.reason} class="text-fg-muted" />
                </div>
              {:else}
                <p class="text-xs italic text-fg-subtle">No reason recorded.</p>
              {/if}

              {#each row.mergedWith as neighbor (neighbor.id)}
                <p data-history-neighbor="merged" class="flex items-center gap-1.5 text-xs text-fg-muted">
                  <GitMerge class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
                  <span>Merged with</span>
                  <a class="truncate text-accent hover:underline" href={resolve(recordHref(neighbor.id))}>“{neighbor.title}”</a>
                </p>
              {/each}

              {#each row.supersedes as { link, predecessor } (link.predecessor)}
                {#if link.diff}
                  {@const stats = diffStats(fromRecordDiffLines(link.diff.lines))}
                  <DisclosureRow data-history-diff={link.predecessor} summaryClass="py-1.5 text-xs text-fg-muted">
                    {#snippet summary()}
                      Changes from “{predecessor.title}”
                      <span class="ml-1 font-mono text-fg-subtle">+{stats.added} −{stats.removed}</span>
                    {/snippet}
                    <RecordDiffView diff={link.diff} />
                  </DisclosureRow>
                {/if}
              {/each}
            </div>
          {/if}

          {#each row.alsoBecame as neighbor (neighbor.id)}
            <p data-history-neighbor="split" class="mt-1 flex items-center gap-1.5 px-1.5 text-xs text-fg-muted">
              <GitFork class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
              <span>Also became</span>
              <a class="truncate text-accent hover:underline" href={resolve(recordHref(neighbor.id))}>“{neighbor.title}”</a>
            </p>
          {/each}
        </div>
      </li>
    {/each}
  </ol>
</section>
