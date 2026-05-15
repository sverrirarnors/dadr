import type { RecordSummary } from '$lib/api';
import { primaryNavItems, type PrimaryNavHref } from '$lib/navigation/routes';
import { recordSearchText } from '$lib/records/filters';

export type CommandPaletteSection = 'Navigation' | 'Records' | 'Actions';
export type CommandPaletteAction =
  | 'new-candidate'
  | 'run-validation'
  | 'toggle-theme'
  | 'reload-snapshot'
  | 'open-settings';

export type CommandPaletteRoute = PrimaryNavHref;

export interface CommandPaletteItem {
  id: string;
  section: CommandPaletteSection;
  label: string;
  detail?: string;
  keywords: string;
  route?: CommandPaletteRoute;
  recordId?: string;
  action?: CommandPaletteAction;
}

const NAVIGATION: CommandPaletteItem[] = primaryNavItems.map((item) =>
  navigationItem(item.href, `Go to ${item.label}`),
);

const ACTIONS: CommandPaletteItem[] = [
  actionItem('new-candidate', 'New Candidate', 'Draft a decision manually'),
  actionItem('run-validation', 'Run Validation', 'Run project validation'),
  actionItem('toggle-theme', 'Toggle Theme', 'Cycle system, light, and dark'),
  actionItem('reload-snapshot', 'Reload Snapshot', 'Refresh records and diagnostics'),
  actionItem('open-settings', 'Open Settings', 'Project and agent settings'),
];

export function buildCommandPaletteItems(records: RecordSummary[]): CommandPaletteItem[] {
  return [
    ...NAVIGATION,
    ...records.map((record) => ({
      id: `record:${record.id}`,
      section: 'Records' as const,
      label: record.title,
      detail: record.path,
      keywords: recordSearchText(record),
      recordId: record.id,
    })),
    ...ACTIONS,
  ];
}

export function filterCommandPaletteItems(
  items: CommandPaletteItem[],
  query: string,
): CommandPaletteItem[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return items;
  return items.filter((item) =>
    [item.label, item.detail, item.keywords, item.section].join(' ').toLowerCase().includes(normalized),
  );
}

function navigationItem(route: CommandPaletteRoute, label: string): CommandPaletteItem {
  return {
    id: `nav:${route}`,
    section: 'Navigation',
    label,
    route,
    keywords: `${label} ${route}`,
  };
}

function actionItem(
  action: CommandPaletteAction,
  label: string,
  detail: string,
): CommandPaletteItem {
  return {
    id: `action:${action}`,
    section: 'Actions',
    label,
    detail,
    action,
    keywords: `${label} ${detail}`,
  };
}
