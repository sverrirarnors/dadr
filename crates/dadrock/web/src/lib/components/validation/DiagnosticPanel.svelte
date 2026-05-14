<script lang="ts">
  import { AlertCircle, CheckCircle2, Info, TriangleAlert } from 'lucide-svelte';
  import type { Diagnostic, DiagnosticPanel as DiagnosticPanelData } from '$lib/api';
  import { Badge, EmptyState, StackedCard } from '$lib/components/ui';
  import DiagnosticActions from '$lib/components/validation/DiagnosticActions.svelte';
  import type { DiagnosticAction } from '$lib/diagnostics/actions';
  import {
    diagnosticLocation,
    formatDiagnosticCounts,
    panelStatus,
    type DiagnosticStatus,
  } from '$lib/diagnostics';

  interface Props {
    panel: DiagnosticPanelData;
    actionsForDiagnostic?: (diagnostic: Diagnostic) => DiagnosticAction[];
  }

  let { panel, actionsForDiagnostic }: Props = $props();

  const counts = $derived(formatDiagnosticCounts({
    errors: panel.diagnostics.filter((diagnostic) => diagnostic.severity === 'error').length,
    warnings: panel.diagnostics.filter((diagnostic) => diagnostic.severity === 'warning').length,
    infos: panel.diagnostics.filter((diagnostic) => diagnostic.severity === 'info').length,
  }));
  const status = $derived(panelStatus(panel));

  const statusClass: Record<DiagnosticStatus, string> = {
    error: 'border-danger/40 bg-danger/5 text-danger',
    warning: 'border-warning/40 bg-warning/10 text-warning',
    info: 'border-accent/40 bg-accent-subtle text-accent',
    ok: 'border-success/40 bg-success/10 text-success',
  };

  const severityVariant = {
    error: 'danger',
    warning: 'warning',
    info: 'accent',
  } as const;
</script>

<div class="space-y-3">
  <div class="flex items-center justify-between gap-3 rounded-md border px-3 py-2 {statusClass[status]}">
    <div class="flex items-center gap-2 text-sm font-medium">
      {#if status === 'error'}
        <AlertCircle class="h-4 w-4" />
      {:else if status === 'warning'}
        <TriangleAlert class="h-4 w-4" />
      {:else if status === 'info'}
        <Info class="h-4 w-4" />
      {:else}
        <CheckCircle2 class="h-4 w-4" />
      {/if}
      <span>{status === 'ok' ? 'No diagnostics' : status}</span>
    </div>
    <span class="text-xs">{counts}</span>
  </div>

  {#if panel.diagnostics.length === 0}
    <EmptyState>No diagnostics.</EmptyState>
  {:else}
    <div class="space-y-2">
      {#each panel.diagnostics as diagnostic, index (`${diagnostic.code}-${diagnostic.path ?? 'project'}-${index}`)}
        {@const location = diagnosticLocation(diagnostic)}
        {@const actions = actionsForDiagnostic?.(diagnostic) ?? []}
        <StackedCard>
          <div class="flex flex-wrap items-center gap-2">
            <Badge variant={severityVariant[diagnostic.severity]} class="uppercase">
              {diagnostic.severity}
            </Badge>
            <Badge mono>
              {diagnostic.code}
            </Badge>
          </div>
          <p class="mt-2 text-sm text-fg">{diagnostic.message}</p>
          {#if location}
            <p class="mt-2 font-mono text-[11px] text-fg-subtle">{location}</p>
          {/if}
          <DiagnosticActions actions={actions} class="mt-3" />
        </StackedCard>
      {/each}
    </div>
  {/if}
</div>
