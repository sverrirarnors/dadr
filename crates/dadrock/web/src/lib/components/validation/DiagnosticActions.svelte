<script lang="ts">
  import { resolve } from '$app/paths';
  import type { DiagnosticAction } from '$lib/diagnostics/actions';

  interface Props {
    actions: DiagnosticAction[];
    class?: string;
  }

  let { actions, class: className = '' }: Props = $props();
</script>

{#if actions.length}
  <div class={`flex flex-wrap items-center gap-2 ${className}`}>
    {#each actions as action (action.label + action.href)}
      <a
        class="inline-flex h-8 items-center rounded-md border border-border bg-bg px-2.5 text-sm font-medium text-fg transition-colors hover:bg-surface"
        href={resolve(action.href)}
        title={action.title}
        data-diagnostic-action={action.kind}
      >
        {action.label}
      </a>
    {/each}
  </div>
{/if}
