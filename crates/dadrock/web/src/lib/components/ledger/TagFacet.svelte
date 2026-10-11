<script lang="ts">
  import { ChevronDown, ChevronRight } from 'lucide-svelte';
  import { onMount } from 'svelte';
  import type { TagSummary } from '$lib/api';
  import { Button } from '$lib/components/ui';
  import { normalizeLedgerTag } from '$lib/ledger/filters';
  import { buildTagTree, flattenTagTree, formatTagCount } from '$lib/tags/tree';

  let {
    tags,
    selected = null,
    onselect,
  }: {
    tags: TagSummary[];
    /** The active tag filter, with or without a leading '#'. */
    selected?: string | null;
    /** Called with the tag to filter by, or null to clear the filter. */
    onselect: (tag: string | null) => void;
  } = $props();

  const bodyId = $props.id();
  let open = $state(true);

  const rows = $derived(flattenTagTree(buildTagTree(tags)));
  const selectedKey = $derived(selected ? normalizeLedgerTag(selected) : '');

  onMount(() => {
    // Collapsed by default on phones so the list keeps most of the pane.
    open = window.matchMedia('(min-width: 768px)').matches;
  });

  function isActive(pathTag: string): boolean {
    return selectedKey !== '' && normalizeLedgerTag(pathTag) === selectedKey;
  }

  function displayTag(tag: string): string {
    return tag.replace(/^#/, '');
  }
</script>

<section data-ledger-tag-facet class="shrink-0 border-b border-border">
  <div class="flex min-h-8 items-center justify-between gap-2 px-2">
    <button
      type="button"
      aria-expanded={open}
      aria-controls={bodyId}
      class="flex min-w-0 flex-1 items-center gap-1.5 rounded px-1 py-1 text-left text-xs font-medium text-fg-muted hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      onclick={() => (open = !open)}
    >
      {#if open}
        <ChevronDown class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
      {:else}
        <ChevronRight class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
      {/if}
      <span>Tags</span>
      {#if selected}
        <span data-ledger-tag-active class="min-w-0 truncate font-mono font-normal text-accent">
          {displayTag(selected)}
        </span>
      {/if}
    </button>
    {#if selected}
      <Button type="button" variant="ghost" size="sm" class="h-7 shrink-0 px-2 text-xs" onclick={() => onselect(null)}>
        Clear
      </Button>
    {/if}
  </div>

  {#if open}
    <ul id={bodyId} data-ledger-tag-rows class="max-h-56 overflow-auto px-1 pb-1">
      {#each rows as row (row.pathTag)}
        {@const active = isActive(row.pathTag)}
        <li>
          <button
            type="button"
            data-tag={row.pathTag}
            aria-pressed={active}
            class="flex w-full items-center gap-2 rounded py-1 pr-2 text-left text-xs transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            class:bg-accent-subtle={active}
            class:text-fg={active}
            class:text-fg-muted={!active}
            class:hover:bg-surface-2={!active}
            class:hover:text-fg={!active}
            style={`padding-left: ${0.5 + row.depth * 0.75}rem`}
            onclick={() => onselect(active ? null : row.pathTag)}
          >
            <span class="min-w-0 flex-1 truncate font-mono">{displayTag(row.pathTag)}</span>
            <span class="shrink-0 text-[11px] text-fg-subtle">{formatTagCount(row)}</span>
          </button>
        </li>
      {:else}
        <li class="px-2 py-1 text-xs text-fg-subtle">No tags yet.</li>
      {/each}
    </ul>
  {/if}
</section>
