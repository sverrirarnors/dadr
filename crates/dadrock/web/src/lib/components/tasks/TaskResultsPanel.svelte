<script lang="ts">
  import type { Snippet } from 'svelte';
  import { PanelSection } from '$lib/components/ui';

  type Props = {
    runLabel: string;
    resultLabel?: string;
    status?: string;
    actions?: Snippet;
    children?: Snippet;
  };

  let { runLabel, resultLabel = '', status = '', actions, children }: Props = $props();
</script>

<PanelSection data-task-results-panel class="h-full rounded-none border-0 p-0">
  <header class="flex flex-wrap items-start justify-between gap-3 border-b border-border p-4">
    <div>
      <h2 class="text-lg font-semibold">Results</h2>
      <p data-task-result-meta class="mt-1 flex flex-wrap items-center gap-1.5 text-xs text-fg-muted">
        {#if resultLabel}
          <span>{resultLabel}</span>
        {/if}
        {#if status}
          <span>{status}</span>
        {/if}
        <span class="font-mono">{runLabel}</span>
      </p>
    </div>
    {#if actions}
      <div class="flex flex-wrap items-center gap-2">
        {@render actions()}
      </div>
    {/if}
  </header>
  <div class="p-4">
    {@render children?.()}
  </div>
</PanelSection>
