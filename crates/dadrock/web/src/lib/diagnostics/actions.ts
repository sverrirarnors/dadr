import type { Diagnostic, RecordSummary } from '$lib/api';
import { diagnosticLocation } from '$lib/diagnostics';
import { taskPrefillHref } from '$lib/tasks/prefill';

export type DiagnosticActionCategory = 'Project' | 'Records' | 'Links' | 'Settings';

export interface DiagnosticAction {
  label: string;
  href: `/tasks?${string}` | `/records?${string}`;
  title: string;
  kind: 'manual' | 'applyable' | 'investigate';
}

export interface DiagnosticActionSource {
  category: DiagnosticActionCategory;
  diagnostic: Diagnostic;
  records: RecordSummary[];
}

export function diagnosticActions(source: DiagnosticActionSource): DiagnosticAction[] {
  const record = recordForDiagnostic(source.diagnostic, source.records);
  const actions: DiagnosticAction[] = [];
  if (record) {
    actions.push({
      label: 'Open record',
      href: recordEditHref(record),
      title: 'Manual fix: open the affected record in edit mode.',
      kind: 'manual',
    });
    actions.push({
      label: 'Fix record with agent',
      href: taskPrefillHref('edit-record', {
        recordId: record.id,
        path: source.diagnostic.path,
        instructions: diagnosticTaskInstructions('edit-record', source),
      }),
      title: 'Applyable fix: draft a record revision that can be reviewed and applied.',
      kind: 'applyable',
    });
  } else if (source.diagnostic.path) {
    actions.push({
      label: 'Open path',
      href: taskPrefillHref('custom', {
        path: source.diagnostic.path,
      }),
      title: 'Manual fallback: open this path as Custom task context.',
      kind: 'manual',
    });
  }
  actions.push({
    label: 'Investigate with agent',
    href: taskPrefillHref('custom', {
      recordId: record?.id,
      path: source.diagnostic.path,
      instructions: diagnosticTaskInstructions('custom', source),
    }),
    title: record
      ? 'Investigative fix: ask for a report or recommendation without applying a record revision.'
      : 'Investigative fix: use the diagnostic path and message as Custom task context.',
    kind: 'investigate',
  });
  return actions;
}

export function recordForDiagnostic(diagnostic: Diagnostic, records: RecordSummary[]): RecordSummary | null {
  if (diagnostic.record_id) {
    const record = records.find((record) => record.id === diagnostic.record_id);
    if (record) return record;
  }
  if (!diagnostic.path) return null;
  const diagnosticPath = normalizePath(diagnostic.path);
  return (
    records.find((record) => {
      const recordPath = normalizePath(record.path);
      return diagnosticPath === recordPath || diagnosticPath.endsWith(`/${recordPath}`);
    }) ?? null
  );
}

export function recordEditHref(record: RecordSummary): `/records?${string}` {
  return `/records?id=${encodeURIComponent(record.id)}&tab=edit`;
}

function diagnosticTaskInstructions(taskId: 'custom' | 'edit-record', source: DiagnosticActionSource): string {
  const location = diagnosticLocation(source.diagnostic);
  const related = source.diagnostic.related_paths.length
    ? `\nRelated paths: ${source.diagnostic.related_paths.join(', ')}`
    : '';
  const action =
    taskId === 'edit-record'
      ? 'Revise the selected record to resolve this validation issue. Change only what is needed for the diagnostic.'
      : 'Investigate this validation issue and recommend the smallest appropriate fix.';
  return `${action}\n\nCategory: ${source.category}\nSeverity: ${source.diagnostic.severity}\nCode: ${source.diagnostic.code}\nMessage: ${source.diagnostic.message}${location ? `\nLocation: ${location}` : ''}${related}`;
}

function normalizePath(path: string): string {
  return path.replaceAll('\\', '/').replace(/^\/+/, '');
}
