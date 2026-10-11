import type { Snapshot } from '$lib/api';

export const DEFAULT_PROJECT_NAME = 'dadrock';
export const NO_AGENT_LINE = 'No local agent found';

/** The last path segment of the project root, or the binary name when unknown. */
export function projectName(cwd: string | undefined | null): string {
  if (!cwd) return DEFAULT_PROJECT_NAME;
  const trimmed = cwd.replace(/[\\/]+$/, '');
  const base = trimmed.split(/[\\/]/).pop() ?? '';
  return base || DEFAULT_PROJECT_NAME;
}

type AgentSettings = Pick<Snapshot['settings'], 'agent' | 'agent_available'>;

/**
 * The agent line shown in the project popover: the "configured: …" or
 * "auto-detected: …" setting line without its prefix.
 */
export function agentLine(settings: AgentSettings | undefined): string {
  if (!settings?.agent_available) return NO_AGENT_LINE;
  for (const line of settings.agent) {
    const match = /^(?:configured|auto-detected):\s*(.+)$/.exec(line.trim());
    const value = match?.[1]?.trim();
    if (value && value !== 'none') return value;
  }
  return 'Local agent available';
}

export interface ProjectFact {
  key: string;
  label: string;
  value: string;
  mono: boolean;
}

/** The facts listed in the project popover, in display order. */
export function projectFacts(snapshot: Snapshot | undefined): ProjectFact[] {
  if (!snapshot) return [];
  return [
    { key: 'root', label: 'Root', value: snapshot.project.cwd, mono: true },
    { key: 'decisions', label: 'Decisions', value: snapshot.project.decisions_dir, mono: true },
    { key: 'discovery', label: 'Discovery', value: snapshot.project.discovery_source, mono: false },
    { key: 'agent', label: 'Agent', value: agentLine(snapshot.settings), mono: false },
  ];
}
