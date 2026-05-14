<script lang="ts">
  import { resolve } from '$app/paths';
  import { createQuery } from '@tanstack/svelte-query';
  import { ChevronRight, Tags } from 'lucide-svelte';
  import { createApi } from '$lib/api';
  import { Alert, Badge, EmptyState, ListRow } from '$lib/components/ui';
  import { WorkbenchPage, WorkbenchPane } from '$lib/components/workbench';
  import {
    buildTagTree,
    flattenTagTree,
    formatTagCount,
  } from '$lib/tags/tree';

  const api = createApi();
  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  const rows = $derived(snapshot.data ? flattenTagTree(buildTagTree(snapshot.data.tags)) : []);
  const totalOccurrences = $derived(
    snapshot.data
      ? snapshot.data.tags.reduce((total, tag) => total + tag.occurrence_count, 0)
      : 0,
  );

  function tagSearchHref(tag: string): string {
    return `${resolve('/records')}?scope=tag&q=${encodeURIComponent(tag)}`;
  }
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden">
<WorkbenchPage layout="single">
  <WorkbenchPane data-tags-shell-pane class="flex min-h-0 w-full max-w-6xl flex-col overflow-hidden">
    <h1 class="sr-only">Tags</h1>

    {#if snapshot.isPending}
      <p class="p-4 text-sm text-fg-muted">Loading tags...</p>
    {:else if snapshot.isError}
      <Alert class="m-4" variant="danger" title="Failed to load tags">{snapshot.error.message}</Alert>
    {:else if rows.length === 0}
      <div class="p-4">
        <EmptyState>No tags found.</EmptyState>
      </div>
    {:else}
      <div class="p-4 pb-2">
        <h1 class="text-2xl font-semibold">Tags</h1>
        <div class="mt-1 flex items-center gap-2 text-sm text-fg-muted">
          <Tags class="h-4 w-4" />
          <span>
            {snapshot.data.tags.length} {snapshot.data.tags.length === 1 ? 'tag' : 'tags'} · {totalOccurrences} uses
          </span>
        </div>
      </div>

        <div data-tags-pane class="grid min-h-0 flex-1 auto-rows-min grid-cols-1 content-start overflow-auto px-2 pb-4">
          {#each rows as row (row.pathTag)}
            <a
              href={tagSearchHref(row.pathTag)}
              class="block bg-bg px-1.5 py-0.5"
              aria-label={`View records tagged ${row.pathTag}`}
            >
              <ListRow class="flex min-h-7 items-center py-0.5 text-xs">
                <span
                  class="flex min-w-0 items-center gap-2"
                  style={`padding-left: ${row.depth * 1}rem`}
                >
                  {#if row.depth > 0}
                    <ChevronRight class="h-3 w-3 shrink-0 text-fg-subtle" />
                  {/if}
                  <span class="truncate font-mono text-fg">{row.pathTag}</span>
                  {#if row.directRecordCount === 0}
                    <Badge>namespace</Badge>
                  {/if}
                  <span data-tag-count class="shrink-0 text-[11px] text-fg-muted">{formatTagCount(row)}</span>
                </span>
              </ListRow>
            </a>
          {/each}
        </div>
    {/if}
  </WorkbenchPane>
</WorkbenchPage>
</div>
