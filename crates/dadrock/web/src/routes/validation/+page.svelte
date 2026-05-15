<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
  import {
    createApi,
    type Diagnostic,
    type DiagnosticPanel as DiagnosticPanelData,
    type Snapshot,
    type ValidationTarget,
  } from '$lib/api';
  import { Alert } from '$lib/components/ui';
  import { PaneHeader, ResizableWorkbenchSplit, WorkbenchPane } from '$lib/components/workbench';
  import DiagnosticPanel from '$lib/components/validation/DiagnosticPanel.svelte';
  import ValidationIssueDetail, { type ValidationIssueSeverity } from '$lib/components/validation/ValidationIssueDetail.svelte';
  import {
    diagnosticActions,
    type DiagnosticAction,
    type DiagnosticActionCategory,
  } from '$lib/diagnostics/actions';
  import {
    countDiagnostics,
    diagnosticLocation,
    formatDiagnosticCounts,
    validationPanelsFromSnapshot,
    type DiagnosticCounts,
    type ValidationPanelView,
  } from '$lib/diagnostics';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import { toasts } from '$lib/stores/toast.svelte';

  type ValidationTab = 'summary' | 'project' | 'records' | 'links' | 'settings' | 'audit';
  type AuditCategory = 'Project' | 'Records' | 'Links' | 'Settings';

  interface AuditEvent {
    id: string;
    title: string;
    detail: string;
    meta?: string;
    severity: 'error' | 'warning' | 'info' | 'ok';
    category: AuditCategory;
    diagnostic?: Diagnostic;
  }

  const tabs: Array<{ value: ValidationTab; label: string }> = [
    { value: 'summary', label: 'Summary' },
    { value: 'project', label: 'Project' },
    { value: 'records', label: 'Records' },
    { value: 'links', label: 'Links' },
    { value: 'settings', label: 'Settings' },
    { value: 'audit', label: 'Audit' },
  ];
  const validationCategories: Array<{
    tab: ValidationTab;
    label: string;
    category?: AuditCategory;
  }> = [
    { tab: 'summary', label: 'All' },
    { tab: 'project', label: 'Project', category: 'Project' },
    { tab: 'records', label: 'Records', category: 'Records' },
    { tab: 'links', label: 'Links', category: 'Links' },
    { tab: 'settings', label: 'Settings', category: 'Settings' },
    { tab: 'audit', label: 'Audit' },
  ];

  const api = createApi();
  const queryClient = useQueryClient();

  let pendingTarget = $state<ValidationTarget | null>(null);
  let selectedIssueId = $state('');

  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  const selectedTab = $derived(readTab(page.url.searchParams.get('tab')));
  const panels = $derived(snapshot.data ? validationPanelsFromSnapshot(snapshot.data) : []);
  const panelByTab = $derived(panelMap(panels));
  const auditEventsForSnapshot = $derived(snapshot.data ? auditEvents(snapshot.data) : []);
  const visibleIssueEvents = $derived(issueEventsForTab(selectedTab, auditEventsForSnapshot));
  const selectedIssue = $derived(
    visibleIssueEvents.find((event) => event.id === selectedIssueId) ?? visibleIssueEvents[0] ?? null,
  );
  const selectedIssueActions = $derived(selectedIssue ? actionsForIssue(selectedIssue) : []);
  const selectedValidationTarget = $derived(validationTargetForTab(selectedTab));

  $effect(() => {
    if (!visibleIssueEvents.length) {
      selectedIssueId = '';
      return;
    }
    if (!visibleIssueEvents.some((event) => event.id === selectedIssueId)) {
      selectedIssueId = visibleIssueEvents[0]?.id ?? '';
    }
  });

  const validation = createMutation(() => ({
    mutationFn: (target: ValidationTarget) => api.runValidation(target),
    onSuccess: async (response) => {
      toasts.push({ severity: 'info', message: response.message });
      pendingTarget = null;
      await applyMutationResponse(queryClient, response);
    },
    onError: () => {
      pendingTarget = null;
    },
  }));

  function readTab(value: string | null): ValidationTab {
    return tabs.some((tab) => tab.value === value) ? (value as ValidationTab) : 'summary';
  }

  function setTab(tab: ValidationTab) {
    const target = tab === 'summary' ? '/validation' : (`/validation?tab=${tab}` as `/validation?${string}`);
    goto(resolve(target), { replaceState: true, noScroll: true, keepFocus: true });
  }

  function panelMap(
    panels: ValidationPanelView[],
  ): Record<ValidationTab, ValidationPanelView | undefined> {
    return {
      summary: undefined,
      project: panels.find((panel) => panel.target === 'project'),
      records: panels.find((panel) => panel.target === 'records'),
      links: panels.find((panel) => panel.target === 'links'),
      settings: panels.find((panel) => panel.target === 'tags'),
      audit: undefined,
    };
  }

  function runValidation(target: ValidationTarget) {
    if (validation.isPending) return;
    pendingTarget = target;
    validation.mutate(target);
  }

  function auditEvents(data: Snapshot): AuditEvent[] {
    return [
      {
        id: 'project',
        title: 'Project snapshot loaded',
        detail: data.project.status,
        meta: `${data.project.cwd} - ${data.project.discovery_source}`,
        severity: data.project.initialized ? 'ok' : 'warning',
        category: 'Project',
      },
      ...diagnosticEvents('Project', data.project_validation.diagnostics),
      ...diagnosticEvents('Records', data.record_validation.diagnostics),
      ...diagnosticEvents('Links', data.link_health.diagnostics),
      ...diagnosticEvents('Settings', data.settings.diagnostics),
    ];
  }

  function diagnosticEvents(category: AuditCategory, diagnostics: Diagnostic[]): AuditEvent[] {
    return diagnostics.map((diagnostic, index) => ({
      id: `${category}:${diagnostic.code}:${index}`,
      title: diagnostic.code,
      detail: diagnostic.message,
      meta: diagnosticLocation(diagnostic),
      severity: diagnostic.severity,
      category,
      diagnostic,
    }));
  }

  function actionsForIssue(event: AuditEvent): DiagnosticAction[] {
    if (!event.diagnostic) return [];
    return actionsForDiagnostic(event.category, event.diagnostic);
  }

  function actionsForDiagnostic(category: DiagnosticActionCategory, diagnostic: Diagnostic): DiagnosticAction[] {
    return diagnosticActions({
      category,
      diagnostic,
      records: snapshot.data?.records ?? [],
    });
  }

  function categoryCounts(events: AuditEvent[], category: AuditCategory): DiagnosticCounts {
    return countDiagnostics(
      events
        .filter((event) => event.category === category && event.severity !== 'ok')
        .map((event) => ({
          severity: event.severity === 'ok' ? 'info' : event.severity,
          code: event.title,
          message: event.detail,
          path: null,
          record_id: null,
          field: null,
          line: null,
          related_paths: [],
        })),
    );
  }

  function issueEventsForTab(tab: ValidationTab, events: AuditEvent[]): AuditEvent[] {
    if (tab === 'summary') return events.filter((event) => event.severity !== 'ok');
    if (tab === 'audit') return events;
    const category = categoryForTab(tab);
    return category ? events.filter((event) => event.category === category) : events;
  }

  function categoryForTab(tab: ValidationTab): AuditCategory | undefined {
    if (tab === 'project') return 'Project';
    if (tab === 'records') return 'Records';
    if (tab === 'links') return 'Links';
    if (tab === 'settings') return 'Settings';
    return undefined;
  }

  function validationTargetForTab(tab: ValidationTab): ValidationTarget | undefined {
    if (tab === 'project') return 'project';
    if (tab === 'records') return 'records';
    if (tab === 'links') return 'links';
    if (tab === 'settings') return 'tags';
    return undefined;
  }

  function diagnosticCategoryForTab(tab: ValidationTab): DiagnosticActionCategory | undefined {
    return categoryForTab(tab);
  }

  function actionsForSelectedTabDiagnostic(diagnostic: Diagnostic): DiagnosticAction[] {
    const category = diagnosticCategoryForTab(selectedTab);
    return category ? actionsForDiagnostic(category, diagnostic) : [];
  }

  function tabLabel(tab: ValidationTab): string {
    return validationCategories.find((category) => category.tab === tab)?.label ?? 'All';
  }

  function severityClass(severity: ValidationIssueSeverity): string {
    if (severity === 'error') return 'bg-danger/10 text-danger';
    if (severity === 'warning') return 'bg-warning/10 text-warning';
    if (severity === 'ok') return 'bg-success/10 text-success';
    return 'bg-surface-2 text-fg-muted';
  }

  function panelForTab(tab: ValidationTab): ValidationPanelView | undefined {
    return panelByTab[tab];
  }

  function emptyPanel(): DiagnosticPanelData {
    return { counts: {}, diagnostics: [] };
  }

</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden">
<ResizableWorkbenchSplit splitId="validation" handleLabel="Resize validation panes">
  {#snippet left()}
  <WorkbenchPane framed data-validation-nav-pane class="flex min-h-0 flex-col overflow-hidden">
    <PaneHeader title="Issue queue" subtitle={`${visibleIssueEvents.length} visible`} />
    <div class="flex min-h-0 flex-1 flex-col space-y-3 p-3">
      <div class="flex flex-wrap gap-1.5">
        {#each validationCategories as category (category.tab)}
          {@const categoryDiagnosticCounts = category.category
            ? categoryCounts(auditEventsForSnapshot, category.category)
            : undefined}
          {@const visibleCount = category.category
            ? (categoryDiagnosticCounts?.errors ?? 0) + (categoryDiagnosticCounts?.warnings ?? 0) + (categoryDiagnosticCounts?.infos ?? 0)
            : issueEventsForTab(category.tab, auditEventsForSnapshot).filter((event) => event.severity !== 'ok').length}
          <button
            type="button"
            class="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-bg px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface hover:text-fg"
            class:bg-surface={selectedTab === category.tab}
            class:text-fg={selectedTab === category.tab}
            aria-pressed={selectedTab === category.tab}
            onclick={() => setTab(category.tab)}
          >
            <span>{category.label}</span>
            <span class="text-xs text-fg-subtle">{visibleCount}</span>
          </button>
        {/each}
      </div>

      {#if snapshot.isPending}
        <p class="text-sm text-fg-muted">Loading validation diagnostics...</p>
      {:else if snapshot.isError}
        <Alert variant="danger" title="Failed to load validation diagnostics">
          {snapshot.error.message}
        </Alert>
      {:else if visibleIssueEvents.length}
        <ol data-validation-issue-list class="min-h-0 flex-1 divide-y divide-border overflow-auto border-y border-border">
          {#each visibleIssueEvents as event (event.id)}
            <li>
              <button
                type="button"
                class="block w-full py-1 text-left"
                aria-current={selectedIssue?.id === event.id ? 'true' : undefined}
                onclick={() => (selectedIssueId = event.id)}
              >
                <div
                  class={`rounded px-2 py-1.5 transition-colors ${
                    selectedIssue?.id === event.id ? 'bg-accent-subtle' : 'hover:bg-surface'
                  }`}
                >
                  <div class="flex min-w-0 items-center gap-2">
                    <span
                      class={`shrink-0 rounded px-1.5 py-0.5 text-[11px] font-medium ${severityClass(event.severity)}`}
                    >
                      {event.category}
                    </span>
                    <span class="truncate font-mono text-xs text-fg">{event.title}</span>
                  </div>
                </div>
              </button>
            </li>
          {/each}
        </ol>
      {:else}
        <p class="border-y border-border py-6 text-sm text-fg-muted">No validation issues in this view.</p>
      {/if}
    </div>
  </WorkbenchPane>
  {/snippet}

  {#snippet right()}
  <WorkbenchPane framed data-validation-detail-pane class="min-h-0 overflow-auto">
    <PaneHeader title="Issue detail" subtitle={tabLabel(selectedTab)} />
    <div class="p-4">
    <h1 class="sr-only">Validation</h1>

    {#if snapshot.isPending}
      <p class="text-sm text-fg-muted">Loading validation diagnostics...</p>
    {:else if snapshot.isError}
      <Alert class="mt-6" variant="danger" title="Failed to load validation diagnostics">
        {snapshot.error.message}
      </Alert>
    {:else}
      {#if validation.isError}
        <Alert class="mt-6" variant="danger" title="Validation failed">{validation.error.message}</Alert>
      {/if}

      <section class="space-y-4">
        <ValidationIssueDetail
          issue={selectedIssue}
          actions={selectedIssueActions}
          {severityClass}
          validationLabel={selectedValidationTarget === 'links' ? 'link' : (selectedValidationTarget ?? '')}
          validationPending={Boolean(validation.isPending && pendingTarget === selectedValidationTarget)}
          onrun={selectedValidationTarget ? () => runValidation(selectedValidationTarget) : undefined}
        />

        {#if panelForTab(selectedTab)}
          {@const panel = panelForTab(selectedTab)}
          <div class="border-t border-border pt-4">
            <div class="mb-3 flex flex-wrap items-start justify-between gap-3">
              <div>
                <h3 class="text-sm font-semibold text-fg">{panel?.title}</h3>
                <p class="mt-1 text-xs text-fg-muted">{panel?.description}</p>
              </div>
              <p class="text-xs text-fg-subtle">{formatDiagnosticCounts(panel?.counts ?? { errors: 0, warnings: 0, infos: 0 })}</p>
            </div>
            <DiagnosticPanel
              panel={panel?.panel ?? emptyPanel()}
              actionsForDiagnostic={actionsForSelectedTabDiagnostic}
            />
          </div>
        {/if}
      </section>
    {/if}
    </div>
  </WorkbenchPane>
  {/snippet}
</ResizableWorkbenchSplit>
</div>
