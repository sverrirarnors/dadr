import { beforeEach, describe, expect, it } from 'vitest';
import { MockApi, resetMockProject } from '../../src/lib/api/mock';
import { mockId, shortId } from '../../src/lib/api/mock-records';
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

describe('mock file names', () => {
  const pathOf = async (id: string) => (await api.getRecordSource(id)).path;
  const SHORT = shortId(OUTLIERS);

  it('names records after their titles and adds the short id only on a collision', async () => {
    const first = await api.createCandidate({ title: 'Report the sample size!', tags: [], body_markdown: sections });
    const second = await api.createCandidate({ title: 'Report the sample size', tags: [], body_markdown: sections });
    expect(first.changed_paths).toEqual(['dadr/inbox/report-the-sample-size.md']);
    expect(second.changed_paths).toEqual([`dadr/inbox/report-the-sample-size-${SHORT}.md`]);
    await expect(
      api.createCandidate({ title: 'Report the sample size', tags: [], body_markdown: sections }),
    ).rejects.toThrow(/refusing to overwrite/);
  });

  it('keeps file names across folders; a successor takes the name its predecessor leaves', async () => {
    expect(await pathOf(OUTLIERS)).toBe('dadr/ledger/outlier-rule-for-covariates.md');
    const response = await api.createCandidate({
      title: 'Outlier rule for covariates',
      tags: [],
      body_markdown: sections,
      supersedes: [OUTLIERS],
    });
    const successor = response.focus?.selected_id ?? '';
    expect(await pathOf(successor)).toBe('dadr/inbox/outlier-rule-for-covariates.md');

    await api.acceptCandidate(successor);
    expect(await pathOf(successor)).toBe('dadr/ledger/outlier-rule-for-covariates.md');
    expect(await pathOf(OUTLIERS)).toBe('dadr/archive/outlier-rule-for-covariates.md');

    await api.rejectCandidate(INCOME);
    expect(await pathOf(INCOME)).toBe('dadr/archive/winsorize-income-at-p99.md');
  });

  it('adds the short id when the new folder already has the name', async () => {
    const response = await api.createCandidate({ title: 'Use SAS for estimation', tags: [], body_markdown: sections });
    const id = response.focus?.selected_id ?? '';
    await api.rejectCandidate(id);
    expect(await pathOf(SAS)).toBe('dadr/archive/use-sas-for-estimation.md');
    expect(await pathOf(id)).toBe(`dadr/archive/use-sas-for-estimation-${SHORT}.md`);
  });
});

describe('mock corrections in place', () => {
  it('lets the title, body, tags, and other keys of accepted and rejected records change', async () => {
    for (const id of [IMPUTATION, SAS]) {
      const { content } = await api.getRecordSource(id);
      const edited = content
        .replace(/^title: (.*)$/m, 'title: $1 (corrected)')
        .replace(/^tags:\n(?: {2}- .*\n)*/m, 'tags:\n  - topic/new\n')
        .replace('---\n\n', 'bibliography: refs.bib\n---\n\n')
        .replace('## Consequences\n\n', '## Consequences\n\nCorrected. ');
      const response = await api.saveRecordSource(id, edited);
      const summary = response.snapshot.records.find((record) => record.id === id);
      expect(summary).toMatchObject({ id, tags: ['topic/new'], title: expect.stringMatching(/\(corrected\)$/) });
      expect(summary?.status).toBe(id === SAS ? 'rejected' : 'accepted');
      expect((await api.getRecordSource(id)).content).toContain('bibliography: refs.bib');
    }
  });

  it('refuses to change the id, status, or successors of a decided record', async () => {
    const { content } = await api.getRecordSource(IMPUTATION);
    await expect(
      api.saveRecordSource(IMPUTATION, content.replace('status: accepted', 'status: rejected')),
    ).rejects.toThrow(/`status` cannot change\. To change the decision, supersede it/);
    await expect(api.saveRecordSource(IMPUTATION, content.replace(`id: ${IMPUTATION}`, `id: ${mockId(9999)}`))).rejects.toThrow(
      /`id` cannot change/,
    );
    const listwise = await api.getRecordSource(LISTWISE);
    await expect(
      api.saveRecordSource(LISTWISE, listwise.content.replace(/successors:\n {2}- .*\n/, '')),
    ).rejects.toThrow(/`successors` cannot change/);
    const rejected = await api.getRecordSource(SAS);
    await expect(
      api.saveRecordSource(SAS, rejected.content.replace('status: rejected', 'status: accepted')),
    ).rejects.toThrow(/draft a new candidate/);
  });

  it('keeps the four required sections of an accepted record', async () => {
    const { content } = await api.getRecordSource(IMPUTATION);
    await expect(
      api.saveRecordSource(IMPUTATION, content.replace('## Alternatives', '## Options')),
    ).rejects.toThrow(/`## Alternatives` is missing/);
    // A rejected record keeps its text but not necessarily the sections.
    const rejected = await api.getRecordSource(SAS);
    await expect(api.saveRecordSource(SAS, rejected.content.replace('## Alternatives', '## Options'))).resolves.toBeDefined();
  });

  it('lets candidates change freely', async () => {
    const { content } = await api.getRecordSource(INCOME);
    const response = await api.saveRecordSource(INCOME, content.replace('Winsorize income at p99', 'Winsorize income at p98'));
    expect(response.snapshot.records.find((record) => record.id === INCOME)?.title).toBe('Winsorize income at p98');
  });
});

describe('mock text edits', () => {
  it('lists the working-tree edit, then each commit, newest first, with diffs on request', async () => {
    const history = await api.getRecordHistory(IMPUTATION, { patch: true });
    const version = history.versions.find((entry) => entry.id === IMPUTATION);
    expect(version?.edits).toHaveLength(2);
    const [working, committed] = version?.edits ?? [];
    expect(working).toMatchObject({
      commit: null,
      short_commit: null,
      author: null,
      date: null,
      subject: null,
      title_changed: false,
      body_changed: true,
    });
    expect(working?.diff?.lines).toContainEqual({
      kind: 'added',
      text: 'The full sample is kept; estimates pool across imputations with Rubin’s rules (Rubin 1987).',
    });
    expect(committed).toMatchObject({
      short_commit: 'e3b1c9a',
      author: 'Alice Martin',
      date: '2026-10-01T14:22:05+02:00',
      subject: 'Fix a typo in the imputation decision',
      title_changed: false,
      body_changed: true,
    });
    expect(committed?.diff?.lines.filter((line) => line.kind !== 'context')).toEqual([
      { kind: 'removed', text: 'Impute income with chained equatoins and pool 20 imputations.' },
      { kind: 'added', text: 'Impute income with chained equations and pool 20 imputations.' },
    ]);
    expect((await api.getRecordHistory(IMPUTATION)).versions[0]?.edits?.[0]?.diff).toBeUndefined();
  });

  it('has no edits for unedited records and candidates', async () => {
    const history = await api.getRecordHistory(LISTWISE);
    expect(history.versions.find((entry) => entry.id === LISTWISE)?.edits).toEqual([]);
    expect((await api.getRecordHistory(INCOME)).versions.find((entry) => entry.id === INCOME)?.edits).toEqual([]);
  });

  it('lists a correction saved in place as not committed yet, but not a change of tags alone', async () => {
    const { content } = await api.getRecordSource(POLARS);
    await api.saveRecordSource(POLARS, content.replace(/^tags:\n(?: {2}- .*\n)*/m, 'tags:\n  - kind/library\n'));
    expect((await api.getRecordHistory(POLARS)).versions[0]?.edits).toEqual([]);

    await api.saveRecordSource(POLARS, content.replace('The full panel fits in memory.', 'The full panel fits in memory (32 GB).'));
    const edits = (await api.getRecordHistory(POLARS)).versions[0]?.edits;
    expect(edits).toEqual([expect.objectContaining({ commit: null, body_changed: true, title_changed: false })]);
  });

  it('reports no edits outside a Git repository', async () => {
    resetMockProject('history', { git: false });
    const history = await api.getRecordHistory(IMPUTATION, { patch: true });
    expect(history.versions.map((version) => version.edits)).toEqual([null, null]);
  });
});
