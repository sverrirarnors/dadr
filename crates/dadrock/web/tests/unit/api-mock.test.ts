import { beforeEach, describe, expect, it } from 'vitest';
import { MockApi, resetMockProject } from '../../src/lib/api/mock';
import { mockId } from '../../src/lib/api/mock-records';
import { rewriteTags } from '../../src/lib/records/front-matter';
import { supersedesSection } from '../../src/lib/records/supersedes';

const api = new MockApi();

const LISTWISE = mockId(1001);
const IMPUTATION = mockId(1002);
const OUTLIERS = mockId(2001);
const INCOME = mockId(2002);
const AGES = mockId(2003);
const TRIM = mockId(3001);
const CAP = mockId(3002);
const MERGE = mockId(3003);
const PANDAS = mockId(4001);
const POLARS = mockId(4002);
const STALE = mockId(4003);
const SAS = mockId(5001);

const sections = '## Context\n\nc\n\n## Decision\n\nd\n\n## Alternatives\n\na\n\n## Consequences\n\nq\n';

beforeEach(() => resetMockProject('history'));

describe('mock history and diff', () => {
  it('returns the versions newest first with the reason and the diff of each link', async () => {
    const history = await api.getRecordHistory(LISTWISE, { patch: true });
    expect(history.record).toBe(LISTWISE);
    expect(history.versions.map((version) => [version.title, version.current])).toEqual([
      ['Multiple imputation for income', true],
      ['Listwise deletion', false],
    ]);
    expect(history.versions[0]?.body).not.toContain('## Supersedes');
    expect(history.links).toHaveLength(1);
    expect(history.links[0]).toMatchObject({
      predecessor: LISTWISE,
      successor: IMPUTATION,
      reason: 'Complete cases lose a third of the sample.',
    });
    expect(history.links[0]?.diff).toMatchObject({
      changed: true,
      title: { old: 'Listwise deletion', new: 'Multiple imputation for income' },
      tags_added: ['kind/transformation'],
      tags_removed: [],
    });
    expect((await api.getRecordHistory(LISTWISE)).links[0]?.diff).toBeUndefined();
  });

  it('diffs any two records, from the one compared against', async () => {
    const result = await api.getRecordDiff(INCOME, OUTLIERS);
    expect(result.old.id).toBe(OUTLIERS);
    expect(result.new.id).toBe(INCOME);
    expect(result.diff.lines).toContainEqual({ kind: 'added', text: 'Winsorize income at the 99th percentile.' });
    expect(result.diff.lines.some((line) => line.text === '## Supersedes')).toBe(false);
  });
});

describe('mock lifecycle', () => {
  it('reports named predecessors and stale ones with the decisions now in force', async () => {
    const merge = await api.getRecordLifecycle(MERGE);
    expect(merge.pending_supersedes.map((record) => record.id)).toEqual([TRIM, CAP]);
    expect(merge.stale_supersedes).toEqual([]);

    const stale = await api.getRecordLifecycle(STALE);
    expect(stale.stale_supersedes).toHaveLength(1);
    expect(stale.stale_supersedes[0]?.predecessor.id).toBe(PANDAS);
    expect(stale.stale_supersedes[0]?.current_successors.map((record) => record.id)).toEqual([POLARS]);
  });

  it('accepts a split together: the predecessor gains every half', async () => {
    const response = await api.acceptCandidate(INCOME, [AGES]);
    expect(response.message).toBe('accepted 2 candidates');
    const predecessor = response.snapshot.records.find((record) => record.id === OUTLIERS);
    expect(predecessor).toMatchObject({ superseded: true, path: expect.stringContaining('dadr/archive/') });
    const history = await api.getRecordHistory(INCOME);
    expect(history.versions.map((version) => version.id)).toEqual([INCOME, OUTLIERS]);
    expect(history.neighbors.map((neighbor) => neighbor.id)).toEqual([AGES]);
  });

  it('refuses to accept a stale candidate until it is retargeted', async () => {
    await expect(api.acceptCandidate(STALE)).rejects.toThrow(/already superseded/);
  });

  it('creates a successor of several decisions with one link each and the reason', async () => {
    const response = await api.createCandidate({
      title: 'One rule',
      tags: ['topic/outliers'],
      body_markdown: sections,
      supersedes: [TRIM, CAP],
      supersedes_reason: 'They disagreed.',
    });
    const id = response.focus?.selected_id ?? '';
    const source = await api.getRecordSource(id);
    const section = supersedesSection(source.content.slice(source.content.indexOf('## Supersedes')));
    expect(section).toEqual({
      links: [
        { id: TRIM, label: 'Trim income at p95' },
        { id: CAP, label: 'Cap ages at 99' },
      ],
      reason: 'They disagreed.',
    });
    expect((await api.getRecordLifecycle(id)).pending_supersedes.map((record) => record.id)).toEqual([TRIM, CAP]);
    await expect(
      api.createCandidate({ title: 'x', tags: [], body_markdown: sections, supersedes: PANDAS }),
    ).rejects.toThrow(/already superseded/);
  });
});

describe('mock frozen records', () => {
  it('lets the tags of accepted and rejected records change', async () => {
    for (const id of [IMPUTATION, SAS]) {
      const { content } = await api.getRecordSource(id);
      const response = await api.saveRecordSource(id, rewriteTags(content, ['topic/new']));
      expect(response.snapshot.records.find((record) => record.id === id)?.tags).toEqual(['topic/new']);
    }
  });

  it('refuses to change the title or body of an accepted or rejected record', async () => {
    const { content } = await api.getRecordSource(IMPUTATION);
    await expect(api.saveRecordSource(IMPUTATION, `${content}\nMore.\n`)).rejects.toThrow(/body cannot change/);
    await expect(
      api.saveRecordSource(IMPUTATION, content.replace('title: Multiple imputation for income', 'title: Other')),
    ).rejects.toThrow(/title cannot change/);
    const rejected = await api.getRecordSource(SAS);
    await expect(api.saveRecordSource(SAS, `${rejected.content}x`)).rejects.toThrow(/draft a new candidate/);
  });

  it('lets candidates change freely', async () => {
    const { content } = await api.getRecordSource(INCOME);
    const response = await api.saveRecordSource(INCOME, content.replace('Winsorize income at p99', 'Winsorize income at p98'));
    expect(response.snapshot.records.find((record) => record.id === INCOME)?.title).toBe('Winsorize income at p98');
  });
});
