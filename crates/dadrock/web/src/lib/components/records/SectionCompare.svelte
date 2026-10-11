<script lang="ts">
  import RenderedMarkdown from '$lib/components/inbox/RenderedMarkdown.svelte';
  import type { SectionComparison } from '$lib/records/sections';

  /**
   * A merge, section by section: for each required section, Decision first,
   * each predecessor's text followed by the new text.
   */
  let { comparisons }: { comparisons: SectionComparison[] } = $props();
</script>

<div data-section-compare class="space-y-5">
  {#each comparisons as comparison (comparison.section)}
    <section data-section-compare-section={comparison.section} class="space-y-2">
      <h4 class="text-sm font-semibold text-fg">{comparison.section}</h4>
      {#each comparison.predecessors as predecessor (predecessor.id)}
        <div class="rounded-md border border-border bg-surface/50 px-3 py-2">
          <p class="text-[11px] font-medium uppercase tracking-wide text-fg-subtle">{predecessor.title}</p>
          {#if predecessor.text}
            <RenderedMarkdown markdown={predecessor.text} class="mt-1 text-fg-muted" />
          {:else}
            <p class="mt-1 text-sm italic text-fg-subtle">No {comparison.section} section.</p>
          {/if}
        </div>
      {/each}
      <div data-section-compare-new class="rounded-md border border-accent/40 bg-accent-subtle/40 px-3 py-2">
        <p class="text-[11px] font-medium uppercase tracking-wide text-accent">New</p>
        {#if comparison.successor}
          <RenderedMarkdown markdown={comparison.successor} class="mt-1" />
        {:else}
          <p class="mt-1 text-sm italic text-fg-subtle">No {comparison.section} section.</p>
        {/if}
      </div>
    </section>
  {/each}
</div>
