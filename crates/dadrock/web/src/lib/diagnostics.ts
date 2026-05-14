import type { Diagnostic, DiagnosticPanel, ValidationTarget } from './api';

export interface DiagnosticCounts {
  errors: number;
  warnings: number;
  infos: number;
}

export type DiagnosticStatus = 'error' | 'warning' | 'info' | 'ok';

export interface ValidationPanelView {
  target: ValidationTarget;
  title: string;
  description: string;
  panel: DiagnosticPanel;
  counts: DiagnosticCounts;
  status: DiagnosticStatus;
}

interface ValidationPanelSource {
  project_validation: DiagnosticPanel;
  record_validation: DiagnosticPanel;
  link_health: DiagnosticPanel;
  settings: {
    diagnostics: Diagnostic[];
  };
}

export function countDiagnostics(diagnostics: Diagnostic[]): DiagnosticCounts {
  return diagnostics.reduce<DiagnosticCounts>(
    (counts, diagnostic) => {
      if (diagnostic.severity === 'error') counts.errors += 1;
      else if (diagnostic.severity === 'warning') counts.warnings += 1;
      else counts.infos += 1;
      return counts;
    },
    { errors: 0, warnings: 0, infos: 0 },
  );
}

export function panelStatus(panel: DiagnosticPanel): DiagnosticStatus {
  const counts = countDiagnostics(panel.diagnostics);
  if (counts.errors > 0) return 'error';
  if (counts.warnings > 0) return 'warning';
  if (counts.infos > 0) return 'info';
  return 'ok';
}

export function diagnosticLocation(diagnostic: Diagnostic): string {
  const parts = [
    diagnostic.path ? baseName(diagnostic.path) : '',
    diagnostic.line ? `line ${diagnostic.line}` : '',
    diagnostic.field,
    diagnostic.related_paths.length ? `related: ${diagnostic.related_paths.map(baseName).join(', ')}` : '',
  ].filter(Boolean);
  return parts.join(' - ');
}

function baseName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}

export function formatDiagnosticCounts(counts: DiagnosticCounts): string {
  return `${counts.errors} errors, ${counts.warnings} warnings, ${counts.infos} info`;
}

export function validationPanelsFromSnapshot(snapshot: ValidationPanelSource): ValidationPanelView[] {
  const tagsPanel: DiagnosticPanel = {
    counts: {},
    diagnostics: snapshot.settings.diagnostics,
  };
  const panelDefinitions: Array<Pick<ValidationPanelView, 'target' | 'title' | 'description' | 'panel'>> = [
    {
      target: 'project',
      title: 'Project',
      description: 'Project structure, graph consistency, lifecycle state, and cross-record rules.',
      panel: snapshot.project_validation,
    },
    {
      target: 'records',
      title: 'Records',
      description: 'Front matter, required fields, parse errors, and per-record structure.',
      panel: snapshot.record_validation,
    },
    {
      target: 'links',
      title: 'Links',
      description: 'DADR wikilinks and local Markdown links.',
      panel: snapshot.link_health,
    },
    {
      target: 'tags',
      title: 'Tags & Settings',
      description: 'Tag shape, tag metadata, config, templates, and local settings.',
      panel: tagsPanel,
    },
  ];
  return panelDefinitions.map((panel) => ({
    ...panel,
    counts: countDiagnostics(panel.panel.diagnostics),
    status: panelStatus(panel.panel),
  }));
}
