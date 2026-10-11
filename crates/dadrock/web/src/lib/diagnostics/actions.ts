import type { Diagnostic, RecordSummary } from '$lib/api';
import { ledgerUrl, type LedgerHref } from '$lib/navigation/routes';

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

/** Opens the affected record in the Ledger with the editor showing. */
export function recordEditHref(record: RecordSummary): LedgerHref {
  return ledgerUrl({ id: record.id, tab: 'edit', history: record.status !== 'accepted' || record.superseded });
}

/** Instructions handed to the agent when asked to fix a diagnostic. */
export function diagnosticFixInstructions(category: string, diagnostic: Diagnostic): string {
  const location = [diagnostic.path, diagnostic.line ? `line ${diagnostic.line}` : '', diagnostic.field]
    .filter(Boolean)
    .join(' ');
  const related = diagnostic.related_paths.length
    ? `\nRelated paths: ${diagnostic.related_paths.join(', ')}`
    : '';
  return `Revise this record to resolve the validation issue below. Change only what the issue needs.\n\nCategory: ${category}\nSeverity: ${diagnostic.severity}\nCode: ${diagnostic.code}\nMessage: ${diagnostic.message}${location ? `\nLocation: ${location}` : ''}${related}`;
}

function normalizePath(path: string): string {
  return path.replaceAll('\\', '/').replace(/^\/+/, '');
}
