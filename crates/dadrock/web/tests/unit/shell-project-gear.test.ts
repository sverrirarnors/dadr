import { describe, expect, it } from 'vitest';
import type { Snapshot } from '../../src/lib/api';
import { agentLine, projectFacts, projectName } from '../../src/lib/shell/project-gear';

describe('projectName', () => {
  it('uses the last path segment of the project root', () => {
    expect(projectName('/tmp/dadr-mock')).toBe('dadr-mock');
    expect(projectName('/home/me/research/thesis/')).toBe('thesis');
    expect(projectName('C:\\Users\\me\\analysis')).toBe('analysis');
  });

  it('falls back to the binary name when nothing is known', () => {
    expect(projectName(undefined)).toBe('dadrock');
    expect(projectName(null)).toBe('dadrock');
    expect(projectName('')).toBe('dadrock');
    expect(projectName('/')).toBe('dadrock');
  });
});

describe('agentLine', () => {
  it('strips the prefix from the configured or auto-detected line', () => {
    expect(
      agentLine({
        agent: ['mode: local ACP agent adaptor', 'auto-detected: mock-agent (mock-agent --acp)'],
        agent_available: true,
      }),
    ).toBe('mock-agent (mock-agent --acp)');
    expect(
      agentLine({
        agent: ['mode: local ACP agent adaptor', 'configured: claude-agent-acp'],
        agent_available: true,
      }),
    ).toBe('claude-agent-acp');
  });

  it('reports when no agent is available', () => {
    expect(agentLine({ agent: ['auto-detected: none'], agent_available: false })).toBe(
      'No local agent found',
    );
    expect(agentLine(undefined)).toBe('No local agent found');
  });

  it('still says something when the lines are unexpected', () => {
    expect(agentLine({ agent: ['auto-detected: none'], agent_available: true })).toBe(
      'Local agent available',
    );
    expect(agentLine({ agent: [], agent_available: true })).toBe('Local agent available');
  });
});

describe('projectFacts', () => {
  it('lists root, decisions, discovery, and agent in order', () => {
    const snapshot = {
      project: {
        cwd: '/tmp/dadr-mock',
        decisions_dir: '/tmp/dadr-mock/dadr/accepted',
        initialized: true,
        discovery_source: 'directory',
        status: 'ok',
      },
      settings: { agent: ['configured: claude-agent-acp'], agent_available: true },
    } as unknown as Snapshot;

    expect(projectFacts(snapshot)).toEqual([
      { key: 'root', label: 'Root', value: '/tmp/dadr-mock', mono: true },
      { key: 'decisions', label: 'Decisions', value: '/tmp/dadr-mock/dadr/accepted', mono: true },
      { key: 'discovery', label: 'Discovery', value: 'directory', mono: false },
      { key: 'agent', label: 'Agent', value: 'claude-agent-acp', mono: false },
    ]);
  });

  it('is empty before the snapshot loads', () => {
    expect(projectFacts(undefined)).toEqual([]);
  });
});
