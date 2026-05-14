<script lang="ts">
  import { RefreshCw } from 'lucide-svelte';
  import DiagnosticActions from '$lib/components/validation/DiagnosticActions.svelte';
  import type { DiagnosticAction } from '$lib/diagnostics/actions';

  export type ValidationIssueSeverity = 'error' | 'warning' | 'info' | 'ok';

  export interface ValidationIssueView {
    title: string;
    detail: string;
    meta?: string;
    severity: ValidationIssueSeverity;
    category: string;
  }

  interface Props {
    issue: ValidationIssueView | null;
    actions: DiagnosticAction[];
    severityClass: (severity: ValidationIssueSeverity) => string;
    validationLabel?: string;
    validationPending?: boolean;
    onrun?: (() => void) | undefined;
  }

  let {
    issue,
    actions,
    severityClass,
    validationLabel = '',
    validationPending = false,
    onrun,
  }: Props = $props();
</script>

<header class="flex flex-wrap items-start justify-between gap-3 border-b border-border pb-4">
  <div class="min-w-0">
    {#if issue}
      <div class="flex flex-wrap items-center gap-2">
        <span class={`rounded px-1.5 py-0.5 text-[11px] font-medium uppercase ${severityClass(issue.severity)}`}>
          {issue.severity}
        </span>
        <span class="font-mono text-xs text-fg-subtle">{issue.category}</span>
      </div>
      <h2 class="mt-2 break-words font-mono text-base font-semibold text-fg">{issue.title}</h2>
    {:else}
      <h2 class="text-base font-semibold text-fg">No issue selected</h2>
    {/if}
  </div>
  <div class="flex flex-wrap items-center justify-end gap-2">
    <DiagnosticActions actions={actions} />
    {#if onrun}
      <button
        type="button"
        class="inline-flex h-8 items-center gap-1.5 rounded-md bg-accent px-2.5 text-sm font-medium text-accent-fg transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
        aria-label={`Run ${validationLabel} validation`}
        disabled={validationPending}
        onclick={onrun}
      >
        <RefreshCw class={`h-3.5 w-3.5 ${validationPending ? 'animate-spin' : ''}`} />
        <span>{validationPending ? 'Running' : 'Run'}</span>
      </button>
    {/if}
  </div>
</header>

{#if issue}
  <div>
    <p class="text-sm leading-6 text-fg">{issue.detail}</p>
    {#if issue.meta}
      <p class="mt-3 break-words font-mono text-xs leading-5 text-fg-muted">{issue.meta}</p>
    {/if}
  </div>
{:else}
  <p class="text-sm text-fg-muted">No validation issues in this view.</p>
{/if}
