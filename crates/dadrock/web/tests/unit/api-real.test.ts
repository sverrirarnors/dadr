import { describe, expect, it, vi } from 'vitest';
import { RealApi } from '../../src/lib/api/real';

const emptyPanel = { counts: {}, diagnostics: [] };
const validSnapshot = {
  project: {
    cwd: '/tmp/proj',
    decisions_dir: '/tmp/proj/dadr/accepted',
    initialized: true,
    discovery_source: 'directory',
    status: 'Project discovered.',
  },
  records: [],
  counts: { total: 0, candidates: 0, accepted: 0, rejected: 0, superseded: 0 },
  tags: [],
  project_validation: emptyPanel,
  record_validation: emptyPanel,
  link_health: emptyPanel,
  settings: {
    config: [],
    templates: [],
    agent: [],
    tags: [],
    tag_counts: {},
    diagnostics: [],
  },
  command_catalog: {},
};

describe('RealApi.getSnapshot', () => {
  it('parses a valid response with the snapshot schema', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(validSnapshot), {
        headers: { 'content-type': 'application/json' },
      }),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);
    const snapshot = await api.getSnapshot();
    expect(snapshot.records).toEqual([]);
    expect(fakeFetch).toHaveBeenCalledWith('http://example.test/api/project', expect.any(Object));
  });

  it('throws when the response shape is invalid', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ wrong: 'shape' }), {
        headers: { 'content-type': 'application/json' },
      }),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);
    await expect(api.getSnapshot()).rejects.toThrow();
  });

  it('throws on non-2xx status', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(new Response('boom', { status: 500 }));
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);
    await expect(api.getSnapshot()).rejects.toThrow();
  });
});

describe('RealApi.getRecordSource', () => {
  it('parses a valid record source response', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          id: '0193fb1f-a3a0-7000-8000-000000000001',
          path: 'dadr/accepted/0001-use-polars.md',
          content: '---\nid: 0193fb1f-a3a0-7000-8000-000000000001\n---\n\n# Use Polars',
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);
    const source = await api.getRecordSource('0193fb1f-a3a0-7000-8000-000000000001');
    expect(source.path).toBe('dadr/accepted/0001-use-polars.md');
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/records/0193fb1f-a3a0-7000-8000-000000000001/source',
      expect.any(Object),
    );
  });

  it('throws when the record source shape is invalid', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ id: '0193fb1f-a3a0-7000-8000-000000000001' }), {
        headers: { 'content-type': 'application/json' },
      }),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);
    await expect(api.getRecordSource('0193fb1f-a3a0-7000-8000-000000000001')).rejects.toThrow();
  });
});

describe('RealApi.getRecordLifecycle', () => {
  it('parses record lifecycle with predecessors and successors', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          record: {
            id: '0193fb1f-a3a0-7000-8000-000000000001',
            id_prefix: '0193fb1fa3a0',
            simple_record_id: '0193fb1fa3a070008000000000000001',
            title: 'Use Polars over pandas',
            status: 'accepted',
            superseded: true,
            path: 'dadr/accepted/0001-use-polars.md',
            preview: 'Use Polars.',
            tags: ['#dadr/data/processing'],
            timestamp_ms: 1730000000000,
          },
          predecessors: [],
          successors: [
            {
              id: '0193fb1f-a3a0-7000-8000-000000000002',
              id_prefix: '0193fb1fa3a0',
              simple_record_id: '0193fb1fa3a070008000000000000002',
              title: 'Use Polars lazy frames',
              status: 'accepted',
              superseded: false,
              path: 'dadr/accepted/0002-use-polars-lazy.md',
              preview: 'Use lazy frames.',
              tags: ['#dadr/data/processing'],
              timestamp_ms: 1730000100000,
            },
          ],
          diagnostics: [],
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    const response = await api.getRecordLifecycle('0193fb1f-a3a0-7000-8000-000000000001');

    expect(response.record.superseded).toBe(true);
    expect(response.successors[0]?.title).toBe('Use Polars lazy frames');
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/records/0193fb1f-a3a0-7000-8000-000000000001/status',
      expect.objectContaining({ method: 'GET' }),
    );
  });
});

describe('RealApi.saveRecordSource', () => {
  it('posts edited content and parses the mutation response', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          message: 'saved 0193fb1f-a3a0-7000-8000-000000000001',
          changed_paths: ['dadr/accepted/0001-use-polars.md'],
          focus: {
            selected_id: '0193fb1f-a3a0-7000-8000-000000000001',
            status_filter: 'accepted',
          },
          snapshot: validSnapshot,
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);
    const response = await api.saveRecordSource(
      '0193fb1f-a3a0-7000-8000-000000000001',
      'edited markdown',
    );
    expect(response.message).toContain('saved');
    expect(response.changed_paths).toEqual(['dadr/accepted/0001-use-polars.md']);
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/records/0193fb1f-a3a0-7000-8000-000000000001/source',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ content: 'edited markdown' }),
      }),
    );
  });

  it('throws when the save response shape is invalid', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ message: 'saved' }), {
        headers: { 'content-type': 'application/json' },
      }),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);
    await expect(
      api.saveRecordSource('0193fb1f-a3a0-7000-8000-000000000001', 'edited markdown'),
    ).rejects.toThrow();
  });
});

describe('RealApi.applyRecordRevision', () => {
  it('posts a record revision and parses the mutation response', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          message: 'applied revision 0193fb1f-a3a0-7000-8000-000000000002',
          changed_paths: ['dadr/candidates/0002-preregister.md'],
          focus: {
            selected_id: '0193fb1f-a3a0-7000-8000-000000000002',
            status_filter: 'candidate',
          },
          snapshot: validSnapshot,
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    const response = await api.applyRecordRevision({
      record_id: '0193fb1f-a3a0-7000-8000-000000000002',
      title: 'Adopt pre-registration for confirmatory studies',
      tags: ['#dadr/methodology', '#writer/agent'],
      body_markdown: '## Context\nRevision.\n\n## Decision\nPre-register confirmatory studies.',
    });

    expect(response.focus?.selected_id).toBe('0193fb1f-a3a0-7000-8000-000000000002');
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/records/0193fb1f-a3a0-7000-8000-000000000002/revision',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({
          title: 'Adopt pre-registration for confirmatory studies',
          tags: ['#dadr/methodology', '#writer/agent'],
          body_markdown: '## Context\nRevision.\n\n## Decision\nPre-register confirmatory studies.',
        }),
      }),
    );
  });
});

describe('RealApi.runValidation', () => {
  it('posts to the selected validation endpoint and parses the mutation response', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          message: 'link validation: 1 error(s), 0 warning(s), 0 info',
          changed_paths: [],
          snapshot: validSnapshot,
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    const response = await api.runValidation('links');

    expect(response.message).toContain('link validation');
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/validate/links',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({}),
      }),
    );
  });

  it('throws when the validation response shape is invalid', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ message: 'project validation: ok' }), {
        headers: { 'content-type': 'application/json' },
      }),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    await expect(api.runValidation('project')).rejects.toThrow();
  });
});

describe('RealApi.getCandidateTemplate', () => {
  it('parses the candidate template response', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          name: 'candidate',
          source: 'built_in',
          path: null,
          body: '## Context\n\n## Decision\n',
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    const response = await api.getCandidateTemplate();

    expect(response.body).toContain('## Decision');
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/templates/candidate',
      expect.objectContaining({ method: 'GET' }),
    );
  });
});

describe('RealApi.createCandidate', () => {
  it('posts manual candidate content and parses the mutation response', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          message: 'created candidate 0193fb1f-a3a0-7000-8000-000000000003',
          changed_paths: ['dadr/candidates/0003-use-duckdb.md'],
          focus: {
            selected_id: '0193fb1f-a3a0-7000-8000-000000000003',
            status_filter: 'candidate',
          },
          snapshot: validSnapshot,
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    const response = await api.createCandidate({
      title: 'Use DuckDB for local joins',
      tags: ['#dadr/data/processing'],
      body_markdown: '## Context\n\n## Decision\nUse DuckDB.',
    });

    expect(response.focus?.selected_id).toBe('0193fb1f-a3a0-7000-8000-000000000003');
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/records',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({
          title: 'Use DuckDB for local joins',
          tags: ['#dadr/data/processing'],
          body_markdown: '## Context\n\n## Decision\nUse DuckDB.',
        }),
      }),
    );
  });
});

describe('RealApi.draftCandidate', () => {
  it('posts an agent draft request and parses the mutation response', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          message: 'drafted candidate 0193fb1f-a3a0-7000-8000-000000000004',
          changed_paths: ['dadr/candidates/0004-agent-draft.md'],
          snapshot: validSnapshot,
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    const response = await api.draftCandidate({
      prompt: 'Draft a decision about local joins.',
      prompt_packs: ['dadr'],
      write: true,
    });

    expect(response.message).toContain('drafted candidate');
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/agent/draft',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({
          prompt: 'Draft a decision about local joins.',
          prompt_packs: ['dadr'],
          write: true,
        }),
      }),
    );
  });

  it('includes the server diagnostic when an agent draft fails', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          error:
            'agent draft failed: ACP agent candidate body is missing required section(s): Decision',
        }),
        {
          status: 400,
          headers: { 'content-type': 'application/json' },
        },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    await expect(
      api.draftCandidate({
        prompt: 'Draft a decision about local joins.',
        prompt_packs: ['dadr'],
        write: true,
      }),
    ).rejects.toThrow(
      'POST /api/agent/draft failed with 400: agent draft failed: ACP agent candidate body is missing required section(s): Decision',
    );
  });
});

describe('RealApi.initializeProject', () => {
  it('posts to the init endpoint and parses the mutation response', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          message: 'created 8 paths, reused 0 paths',
          changed_paths: [
            'dadr',
            'dadr/accepted',
            'dadr/candidates',
            'dadr/rejected',
            'dadr/templates',
            'dadr/tasks',
            'dadr/runs',
            'dadr/runs/.gitignore',
          ],
          snapshot: validSnapshot,
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    const response = await api.initializeProject();

    expect(response.message).toContain('created');
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/init',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({}),
      }),
    );
  });
});

describe('RealApi candidate lifecycle actions', () => {
  it('posts accept and reject actions to candidate endpoints', async () => {
    const responseBody = {
      message: 'accepted candidate 0193fb1f-a3a0-7000-8000-000000000002',
      changed_paths: ['dadr/accepted/0002-preregister.md'],
      focus: {
        selected_id: '0193fb1f-a3a0-7000-8000-000000000002',
        status_filter: 'accepted',
      },
      snapshot: validSnapshot,
    };
    const fakeFetch = vi
      .fn()
      .mockResolvedValueOnce(
        new Response(JSON.stringify(responseBody), {
          headers: { 'content-type': 'application/json' },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify(responseBody), {
          headers: { 'content-type': 'application/json' },
        }),
      );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    await api.acceptCandidate('0193fb1f-a3a0-7000-8000-000000000002');
    await api.rejectCandidate('0193fb1f-a3a0-7000-8000-000000000002');

    expect(fakeFetch).toHaveBeenNthCalledWith(
      1,
      'http://example.test/api/candidates/0193fb1f-a3a0-7000-8000-000000000002/accept',
      expect.objectContaining({ method: 'POST' }),
    );
    expect(fakeFetch).toHaveBeenNthCalledWith(
      2,
      'http://example.test/api/candidates/0193fb1f-a3a0-7000-8000-000000000002/reject',
      expect.objectContaining({ method: 'POST' }),
    );
  });
});

describe('RealApi.supersedeRecords', () => {
  it('posts selected successors as one supersede request', async () => {
    const responseBody = {
      message: 'record superseded',
      changed_paths: ['dadr/accepted/old.md', 'dadr/accepted/new.md'],
      snapshot: validSnapshot,
    };
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(responseBody), {
        headers: { 'content-type': 'application/json' },
      }),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    const response = await api.supersedeRecords({
      from: '0193fb1f-a3a0-7000-8000-000000000001',
      to: [
        '0193fb1f-a3a0-7000-8000-000000000003',
        '0193fb1f-a3a0-7000-8000-000000000004',
      ],
    });

    expect(response.message).toBe('record superseded');
    expect(fakeFetch).toHaveBeenCalledTimes(1);
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/records/supersede',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({
          from: '0193fb1f-a3a0-7000-8000-000000000001',
          to: [
            '0193fb1f-a3a0-7000-8000-000000000003',
            '0193fb1f-a3a0-7000-8000-000000000004',
          ],
        }),
      }),
    );
  });

  it('requires at least one successor record', async () => {
    const fakeFetch = vi.fn();
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    await expect(
      api.supersedeRecords({
        from: '0193fb1f-a3a0-7000-8000-000000000001',
        to: [],
      }),
    ).rejects.toThrow('Select at least one superseding record');
    expect(fakeFetch).not.toHaveBeenCalled();
  });
});

describe('RealApi task commands', () => {
  it('includes server error details when running a task fails', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ error: 'ACP error: no local ACP agent found' }), {
        status: 400,
        headers: { 'content-type': 'application/json' },
      }),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    await expect(
      api.runTask({
        task_id: 'code-to-record',
        input: { path: 'R/paperinfo' },
      }),
    ).rejects.toThrow('POST /api/tasks/run failed with 400: ACP error: no local ACP agent found');
  });

  it('posts task context when running a task', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          command: 'task run',
          ok: true,
          data: {
            id: '0193fb1f-a3a0-7000-8000-000000000001',
            task_id: 'code-to-record',
            task_source: { kind: 'builtin', id: 'builtin:code-to-record' },
            task_version: 1,
            status: 'created',
            created_at: '2026-05-10T12:00:00Z',
            updated_at: '2026-05-10T12:00:00Z',
            run_dir: '/tmp/dadr/dadr/runs/code-to-record/0193fb1f-a3a0-7000-8000-000000000001',
          },
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    await api.runTask({
      task_id: 'code-to-record',
      input: { path: 'crates/dadrock' },
      context: {
        instructions: 'Focus on MCP task APIs.',
        paths: ['crates/dadrock/src/mcp.rs'],
        records: ['0193fb1f-a3a0-7000-8000-000000000002'],
        text: 'Reviewer asked for a fix button.',
      },
    });

    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/tasks/run',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({
          task_id: 'code-to-record',
          input: { path: 'crates/dadrock' },
          context: {
            instructions: 'Focus on MCP task APIs.',
            paths: ['crates/dadrock/src/mcp.rs'],
            records: ['0193fb1f-a3a0-7000-8000-000000000002'],
            text: 'Reviewer asked for a fix button.',
          },
        }),
      }),
    );
  });

  it('parses command envelopes when empty metadata fields are omitted', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          command: 'task list',
          ok: true,
          data: {
            tasks: [
              {
                manifest: {
                  id: 'find-conflicts',
                  title: 'Find conflicting decisions',
                  description: 'Scan decisions and candidates for contradictions or unresolved overlap.',
                  version: 1,
                  input: { mode: 'form' },
                  inputs: [
                    {
                      name: 'status_scope',
                      label: 'Records to scan',
                      type: 'select',
                      default: 'live',
                      options: ['live', 'all'],
                      required: false,
                    },
                  ],
                  output: { kind: 'conflict_set.v1' },
                  tools: [{ name: 'dadrock.list_records', mode: 'read' }],
                },
                source: { kind: 'builtin', id: 'builtin:find-conflicts' },
                prompt: 'Find conflicts.',
              },
            ],
            diagnostics: [],
          },
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    const listing = await api.listTasks();

    expect(listing.tasks[0]?.manifest.id).toBe('find-conflicts');
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/tasks',
      expect.objectContaining({ method: 'GET' }),
    );
  });

  it('loads task run logs from command envelopes', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          command: 'task log',
          ok: true,
          data: [
            {
              timestamp: '2026-05-10T12:00:00Z',
              message: 'Starting local ACP agent: mock-agent',
            },
          ],
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    const log = await api.getTaskLog('0193fb1f-a3a0-7000-8000-000000000001');

    expect(log[0]?.message).toBe('Starting local ACP agent: mock-agent');
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/task-runs/0193fb1f-a3a0-7000-8000-000000000001/log',
      expect.objectContaining({ method: 'GET' }),
    );
  });

  it('cancels task runs through a command envelope', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          command: 'task cancel',
          ok: true,
          data: {
            id: '0193fb1f-a3a0-7000-8000-000000000001',
            task_id: 'code-to-record',
            task_source: { kind: 'builtin', id: 'builtin:code-to-record' },
            task_version: 1,
            status: 'cancelled',
            created_at: '2026-05-10T12:00:00Z',
            updated_at: '2026-05-10T12:01:00Z',
            run_dir: '/tmp/dadr/dadr/runs/code-to-record/0193fb1f-a3a0-7000-8000-000000000001',
          },
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    const run = await api.cancelTaskRun('0193fb1f-a3a0-7000-8000-000000000001');

    expect(run.status).toBe('cancelled');
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/task-runs/0193fb1f-a3a0-7000-8000-000000000001/cancel',
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('clears task run history through a mutation response', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          message: 'cleared task history',
          changed_paths: ['/tmp/proj/dadr/runs'],
          snapshot: validSnapshot,
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as typeof fetch);

    const response = await api.clearTaskHistory();

    expect(response.message).toBe('cleared task history');
    expect(fakeFetch).toHaveBeenCalledWith(
      'http://example.test/api/task-runs/clear',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({}),
      }),
    );
  });
});
