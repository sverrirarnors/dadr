import { describe, expect, it } from 'vitest';
import type { Diagnostic, DiagnosticPanel } from '../../src/lib/api';
import {
  countDiagnostics,
  diagnosticLocation,
  panelStatus,
  validationPanelsFromSnapshot,
} from '../../src/lib/diagnostics';

const diagnostics: Diagnostic[] = [
  {
    severity: 'error',
    code: 'missing_front_matter',
    message: 'Missing front matter.',
    path: 'dadr/inbox/choice.md',
    record_id: '0193fb1f-a3a0-7000-8000-000000000001',
    field: 'title',
    line: 7,
    related_paths: [],
  },
  {
    severity: 'warning',
    code: 'stale_link',
    message: 'Link target is stale.',
    path: 'dadr/ledger/other.md',
    record_id: null,
    field: null,
    line: null,
    related_paths: ['dadr/ledger/choice.md'],
  },
  {
    severity: 'info',
    code: 'note',
    message: 'Informational note.',
    path: null,
    record_id: null,
    field: null,
    line: null,
    related_paths: [],
  },
];

describe('countDiagnostics', () => {
  it('counts diagnostics by severity', () => {
    expect(countDiagnostics(diagnostics)).toEqual({
      errors: 1,
      warnings: 1,
      infos: 1,
    });
  });
});

describe('panelStatus', () => {
  it('uses the highest severity in a diagnostic panel', () => {
    expect(panelStatus({ counts: {}, diagnostics })).toBe('error');
    expect(panelStatus({ counts: {}, diagnostics: diagnostics.slice(1) })).toBe('warning');
    expect(panelStatus({ counts: {}, diagnostics: diagnostics.slice(2) })).toBe('info');
    expect(panelStatus({ counts: {}, diagnostics: [] })).toBe('ok');
  });
});

describe('diagnosticLocation', () => {
  it('joins path, line, field, and related paths into a compact location', () => {
    expect(diagnosticLocation(diagnostics[0]!)).toBe('choice.md - line 7 - title');
    expect(diagnosticLocation(diagnostics[1]!)).toBe(
      'other.md - related: choice.md',
    );
    expect(diagnosticLocation(diagnostics[2]!)).toBe('');
  });

  it('shows only the basename for absolute diagnostic paths', () => {
    expect(
      diagnosticLocation({
        ...diagnostics[0]!,
        path: '/Users/vincent/repos/harvest_articles/dadr/ledger/exclude-uncertain.md',
      }),
    ).toBe('exclude-uncertain.md - line 7 - title');
  });
});

describe('validationPanelsFromSnapshot', () => {
  it('returns the four Phase 1 validation panels in display order', () => {
    const emptyPanel: DiagnosticPanel = { counts: {}, diagnostics: [] };
    const panels = validationPanelsFromSnapshot({
      project_validation: { counts: {}, diagnostics: [diagnostics[0]!] },
      record_validation: emptyPanel,
      link_health: { counts: {}, diagnostics: [diagnostics[1]!] },
      settings: {
        diagnostics: [diagnostics[2]!],
      },
    });

    expect(panels.map((panel) => panel.target)).toEqual(['project', 'records', 'links', 'tags']);
    expect(panels.map((panel) => panel.title)).toEqual([
      'Project',
      'Records',
      'Links',
      'Tags & Settings',
    ]);
    expect(panels.map((panel) => panel.counts)).toEqual([
      { errors: 1, warnings: 0, infos: 0 },
      { errors: 0, warnings: 0, infos: 0 },
      { errors: 0, warnings: 1, infos: 0 },
      { errors: 0, warnings: 0, infos: 1 },
    ]);
  });
});
