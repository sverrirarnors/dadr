/**
 * The record store behind the mock API: a small in-memory project that
 * follows Dadrock's rules closely enough for unit and browser tests. Records
 * carry their full body, `## Supersedes` sections are parsed like Dadrock
 * parses them, and accepted and rejected records take corrections in place.
 * A record's text at HEAD and the commits that edited it stand in for Git, so
 * the history lists text edits like `dadrock history` does.
 */
import { lineDiff } from '$lib/inbox/diff';
import { frontMatterBlocks, readFrontMatter, splitSource, yamlScalar } from '$lib/records/front-matter';
import { requiredSectionProblems } from '$lib/records/sections';
import { comparableBody, supersedesLink, supersedesSection, hasSupersedesHeading } from '$lib/records/supersedes';
import type {
  HistoryLink,
  HistoryRecord,
  RecordDiff,
  RecordDiffResult,
  RecordEdit,
  RecordHistory,
  RecordLifecycle,
  RecordSummary,
} from './schemas';

type Status = 'candidate' | 'accepted' | 'rejected';

/** What Git compares to find a text edit: the title and the body. */
export interface MockText {
  title: string;
  body: string;
}

/** A commit that edited a record's title or body after it was accepted or rejected. */
export interface MockCommit {
  commit: string;
  author: string;
  /** Author date, ISO 8601. */
  date: string;
  subject: string;
  before: MockText;
  after: MockText;
}

export interface MockRecord {
  id: string;
  title: string;
  status: Status;
  tags: string[];
  successors: string[];
  /** Everything after the closing front matter delimiter. */
  body: string;
  /** Front matter lines for keys outside DADR. */
  extra: string[];
  /** File name; the folder follows the lifecycle. */
  file: string;
  timestamp_ms: number;
  /** The source exactly as last saved, until the lifecycle rewrites the file. */
  raw?: string | undefined;
  /** The title and body at HEAD once accepted or rejected; a working-tree edit differs from it. */
  head?: MockText | undefined;
  /** Commits that edited the title or body since acceptance or rejection, newest first. */
  log?: MockCommit[] | undefined;
}

export type MockScenario = 'default' | 'history';

const ID_PREFIX = '0193fb1f-a3a0-7000-8000-';

export function mockId(suffix: number): string {
  return `${ID_PREFIX}${String(suffix).padStart(12, '0')}`;
}

function sections(context: string, decision: string, alternatives: string, consequences: string): string {
  return `## Context\n\n${context}\n\n## Decision\n\n${decision}\n\n## Alternatives\n\n${alternatives}\n\n## Consequences\n\n${consequences}\n`;
}

function supersedes(links: Array<[number, string]>, reason: string): string {
  return `## Supersedes\n\n${links.map(([id, title]) => `${supersedesLink({ id: mockId(id), title })}\n`).join('')}\n${reason}\n\n`;
}

function seed(
  suffix: number,
  title: string,
  status: Status,
  body: string,
  options: {
    tags?: string[];
    successors?: number[];
    file?: string;
    timestamp?: number;
    /** The body at HEAD when the working tree has an uncommitted edit. */
    head?: string;
    log?: MockCommit[];
  } = {},
): MockRecord {
  return {
    id: mockId(suffix),
    title,
    status,
    tags: options.tags ?? [],
    successors: (options.successors ?? []).map(mockId),
    body: `\n${body}`,
    extra: [],
    file: options.file ?? `${String(suffix).padStart(4, '0')}-${slugify(title)}.md`,
    timestamp_ms: options.timestamp ?? 1730000000000 + suffix * 1000,
    head: status === 'candidate' ? undefined : { title, body: `\n${options.head ?? body}` },
    log: options.log ?? [],
  };
}

function defaultRecords(): MockRecord[] {
  return [
    seed(1, 'Use Polars over pandas', 'accepted', 'Use Polars for tabular transformations.\n', {
      tags: ['#dadr/data/processing'],
      successors: [3],
      file: '0001-use-polars.md',
      timestamp: 1730000000000,
    }),
    seed(2, 'Adopt pre-registration for studies', 'candidate', 'Candidate decision for pre-registration practice.\n', {
      tags: ['#dadr/methodology'],
      file: '0002-preregister.md',
      timestamp: 1730000100000,
    }),
    seed(
      3,
      'Use Polars lazy frames',
      'accepted',
      'Use lazy frames for transformations that can benefit from deferred execution.\n',
      { tags: ['#dadr/data/processing'], file: '0003-use-polars-lazy.md', timestamp: 1730000200000 },
    ),
    seed(4, 'Use Arrow IPC snapshots', 'accepted', sections(
      'Intermediate data moves between the cleaning and modeling steps.',
      'Use Arrow IPC snapshots for portable intermediate data.',
      'Write CSV files between steps.',
      'Snapshots keep column types and load quickly.',
    ), { tags: ['#dadr/data/processing'], file: '0004-use-arrow-ipc.md', timestamp: 1730000300000 }),
  ];
}

/**
 * Decisions with a history, a split waiting for review, a merge, a stale
 * successor, and a rejected record. Browser tests opt in with
 * `localStorage['dadrock:mock-scenario'] = 'history'`.
 */
function historyRecords(): MockRecord[] {
  const missingData = 'Income is missing for a fifth of the respondents.';
  const outliers = 'Income and age have implausible extreme values.';
  // "Multiple imputation for income" was corrected in place twice since it was
  // accepted: a committed spelling fix, then a citation not committed yet.
  const imputation = (decision: string, consequences: string) =>
    supersedes([[1001, 'Listwise deletion']], 'Complete cases lose a third of the sample.') +
    sections(missingData, decision, 'Drop the rows with missing income.', consequences);
  const imputationAccepted = imputation(
    'Impute income with chained equatoins and pool 20 imputations.',
    'The full sample is kept; estimates pool across imputations.',
  );
  const imputationCommitted = imputation(
    'Impute income with chained equations and pool 20 imputations.',
    'The full sample is kept; estimates pool across imputations.',
  );
  const imputationWorking = imputation(
    'Impute income with chained equations and pool 20 imputations.',
    'The full sample is kept; estimates pool across imputations with Rubin’s rules (Rubin 1987).',
  );
  const imputationTitle = 'Multiple imputation for income';
  return [
    seed(1001, 'Listwise deletion', 'accepted', sections(
      missingData,
      'Drop every row with a missing covariate.',
      'Impute the missing values.',
      'The sample shrinks by a third.',
    ), { tags: ['topic/missing-data'], successors: [1002] }),
    seed(1002, imputationTitle, 'accepted', imputationWorking, {
      tags: ['topic/missing-data', 'kind/transformation'],
      head: imputationCommitted,
      log: [
        {
          commit: 'e3b1c9a4f27d8e6b5a0c1d2e3f4a5b6c7d8e9f01',
          author: 'Alice Martin',
          date: '2026-10-01T14:22:05+02:00',
          subject: 'Fix a typo in the imputation decision',
          before: { title: imputationTitle, body: `\n${imputationAccepted}` },
          after: { title: imputationTitle, body: `\n${imputationCommitted}` },
        },
      ],
    }),
    seed(2001, 'Outlier rule for covariates', 'accepted', sections(
      outliers,
      'Winsorize every continuous covariate at the 99th percentile.',
      'Trim the extreme values.',
      'Extreme values are kept but capped.',
    ), { tags: ['topic/outliers'] }),
    seed(2002, 'Winsorize income at p99', 'candidate', supersedes(
      [[2001, 'Outlier rule for covariates']],
      'Income and age need different rules.',
    ) + sections(
      outliers,
      'Winsorize income at the 99th percentile.',
      'Trim income above the 99th percentile.',
      'Income extremes are kept but capped.',
    ), { tags: ['topic/outliers'] }),
    seed(2003, 'Drop implausible ages', 'candidate', supersedes(
      [[2001, 'Outlier rule for covariates']],
      'Ages above 110 are data entry errors, not outliers.',
    ) + sections(
      outliers,
      'Drop respondents whose recorded age is above 110.',
      'Winsorize age with the other covariates.',
      'A handful of respondents leave the sample.',
    ), { tags: ['topic/outliers', 'topic/sample'] }),
    seed(3001, 'Trim income at p95', 'accepted', sections(
      'Income has a long right tail.',
      'Trim income above the 95th percentile.',
      'Keep every income value.',
      'Five percent of the sample is dropped.',
    ), { tags: ['topic/outliers'] }),
    seed(3002, 'Cap ages at 99', 'accepted', sections(
      'Some recorded ages are implausible.',
      'Cap recorded ages at 99.',
      'Drop the implausible ages.',
      'Old respondents are kept with a capped age.',
    ), { tags: ['topic/outliers'] }),
    seed(3003, 'One outlier rule for all covariates', 'candidate', supersedes(
      [[3001, 'Trim income at p95'], [3002, 'Cap ages at 99']],
      'The two rules disagreed at the tails; one rule now covers every continuous covariate.',
    ) + sections(
      'Income and age both have implausible extreme values.',
      'Winsorize every continuous covariate at the 99th percentile.',
      'Keep a separate rule for each covariate.',
      'Extreme values are kept but capped, the same way for every covariate.',
    ), { tags: ['topic/outliers'] }),
    seed(4001, 'Use pandas for data frames', 'accepted', sections(
      'The cleaning scripts need a data frame library.',
      'Use pandas for every data frame.',
      'Use Polars.',
      'The team already knows pandas.',
    ), { tags: ['kind/tooling'], successors: [4002] }),
    seed(4002, 'Use Polars for data frames', 'accepted', supersedes(
      [[4001, 'Use pandas for data frames']],
      'Polars is faster on the full panel.',
    ) + sections(
      'The cleaning scripts need a data frame library.',
      'Use Polars for every data frame.',
      'Keep pandas.',
      'The full panel fits in memory.',
    ), { tags: ['kind/tooling'] }),
    seed(4003, 'Use pandas with Arrow dtypes', 'candidate', supersedes(
      [[4001, 'Use pandas for data frames']],
      'Arrow dtypes fix the memory blow-up.',
    ) + sections(
      'The cleaning scripts need a data frame library.',
      'Use pandas with Arrow-backed dtypes for every data frame.',
      'Use Polars.',
      'The team already knows pandas, and memory use drops.',
    ), { tags: ['kind/tooling'] }),
    seed(5001, 'Use SAS for estimation', 'rejected', sections(
      'The estimation step needs a statistics package.',
      'Use SAS for every model.',
      'Use R.',
      'Licenses would be needed for every analyst.',
    ), { tags: ['kind/tooling'] }),
  ];
}

export function scenarioRecords(scenario: MockScenario): MockRecord[] {
  return scenario === 'history' ? historyRecords() : defaultRecords();
}

export function slugify(title: string): string {
  return (
    title
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '')
      .slice(0, 48) || 'candidate'
  );
}

/** An in-memory project with Dadrock's lifecycle rules. */
export class MockRecordStore {
  readonly records: MockRecord[];
  /** False stands for a project outside a Git repository: histories have no text edits. */
  readonly git: boolean;

  constructor(records: MockRecord[], options: { git?: boolean } = {}) {
    this.records = records;
    this.git = options.git ?? true;
  }

  find(query: string): MockRecord | undefined {
    const needle = query.trim().toLowerCase();
    const compact = needle.replaceAll('-', '');
    return (
      this.records.find((record) => record.id === needle) ??
      this.records.find((record) => compact.length >= 8 && record.id.replaceAll('-', '').startsWith(compact)) ??
      this.records.find((record) => record.title.toLowerCase() === needle)
    );
  }

  get(query: string): MockRecord {
    const record = this.find(query);
    if (!record) throw new Error(`no record matches \`${query}\``);
    return record;
  }

  /** An accepted record whose successors name a record in the project. */
  isSuperseded(record: MockRecord): boolean {
    return record.status === 'accepted' && this.liveSuccessors(record).length > 0;
  }

  liveSuccessors(record: MockRecord): MockRecord[] {
    return record.successors.flatMap((id) => this.records.filter((other) => other.id === id));
  }

  predecessors(record: MockRecord): MockRecord[] {
    return this.records.filter((other) => other.successors.includes(record.id));
  }

  path(record: MockRecord): string {
    const folder =
      record.status === 'candidate' ? 'inbox' : record.status === 'accepted' && !this.isSuperseded(record) ? 'ledger' : 'archive';
    return `dadr/${folder}/${record.file}`;
  }

  summary(record: MockRecord): RecordSummary {
    return {
      id: record.id,
      id_prefix: record.id.replaceAll('-', '').slice(0, 12),
      simple_record_id: record.id.replaceAll('-', ''),
      title: record.title,
      status: record.status,
      superseded: this.isSuperseded(record),
      path: this.path(record),
      preview: record.body.trim() || 'No preview available.',
      tags: [...record.tags],
      timestamp_ms: record.timestamp_ms,
    };
  }

  summaries(): RecordSummary[] {
    return this.records.map((record) => this.summary(record));
  }

  source(record: MockRecord): string {
    if (record.raw !== undefined) return record.raw;
    const lines = ['---', `title: ${record.title}`, `status: ${record.status}`, `id: ${record.id}`];
    if (record.successors.length) lines.push('successors:', ...record.successors.map((id) => `  - ${id}`));
    if (record.tags.length) lines.push('tags:', ...record.tags.map((tag) => `  - ${yamlScalar(tag)}`));
    lines.push(...record.extra, '---');
    return `${lines.join('\n')}\n${record.body}`;
  }

  /**
   * Save a record's source. Candidates change freely. Accepted and rejected
   * records take corrections in place: title, body, tags, and keys outside
   * DADR may change, `id`, `status`, and `successors` may not, and an
   * accepted record keeps its four required sections.
   */
  save(record: MockRecord, content: string): void {
    const fields = readFrontMatter(content);
    const { frontMatter, body } = splitSource(content);
    if (!frontMatter || !fields.id || !fields.title || !fields.status) {
      throw new Error('record markdown is invalid: missing record front matter');
    }
    if (record.status !== 'candidate') {
      const changed = [
        fields.id !== record.id ? '`id`' : '',
        fields.status !== record.status ? '`status`' : '',
        !sameMembers(fields.successors, record.successors) ? '`successors`' : '',
      ].filter(Boolean);
      if (changed.length) {
        const advice =
          record.status === 'accepted'
            ? 'To change the decision, supersede it'
            : 'To revisit it, draft a new candidate';
        throw new Error(
          `\`${record.title}\` is ${record.status}, so its ${changed.join(', ')} cannot change. ${advice}; the title, body, tags, and keys outside DADR can be corrected in place.`,
        );
      }
      const problems = record.status === 'accepted' ? requiredSectionProblems(body) : [];
      if (problems.length) {
        throw new Error(
          `\`${record.title}\` is accepted and keeps its four required sections, each once and not empty: ${problems.join('; ')}.`,
        );
      }
    } else if (fields.id !== record.id || fields.status !== record.status) {
      throw new Error('record id and status cannot be changed in the web editor');
    }
    record.title = fields.title;
    record.tags = fields.tags;
    record.body = body;
    record.extra = frontMatterBlocks(content)
      .filter((block) => !['id', 'title', 'status', 'tags', 'successors'].includes(block.key))
      .flatMap((block) => block.lines);
    record.raw = content;
  }

  /** The records a candidate's `## Supersedes` section names; unknown ids are an error. */
  named(record: MockRecord): MockRecord[] {
    if (record.status !== 'candidate') return [];
    const section = supersedesSection(record.body);
    if (!section) {
      if (hasSupersedesHeading(record.body)) throw new Error('`## Supersedes` is malformed');
      return [];
    }
    return section.links.map((link) => {
      const named = this.records.find((other) => other.id === link.id.toLowerCase());
      if (!named) throw new Error(`\`## Supersedes\` in \`${record.title}\` names \`${link.id}\`, which is not a record in this project`);
      return named;
    });
  }

  /** The decisions in force in place of a record: successors, transitively, that have none of their own. */
  currentSuccessors(record: MockRecord): MockRecord[] {
    const tips: MockRecord[] = [];
    const seen = new Set([record.id]);
    const queue = [...this.liveSuccessors(record)];
    while (queue.length) {
      const next = queue.shift() as MockRecord;
      if (seen.has(next.id)) continue;
      seen.add(next.id);
      const successors = this.liveSuccessors(next);
      if (successors.length) queue.push(...successors);
      else tips.push(next);
    }
    return tips;
  }

  lifecycle(record: MockRecord): RecordLifecycle {
    const pending = (() => {
      try {
        return this.named(record);
      } catch {
        return [];
      }
    })();
    return {
      record: this.summary(record),
      predecessors: this.predecessors(record).map((other) => this.summary(other)),
      successors: this.liveSuccessors(record).map((other) => this.summary(other)),
      pending_supersedes: pending.map((other) => this.summary(other)),
      stale_supersedes: pending
        .filter((other) => this.isSuperseded(other))
        .map((other) => ({
          predecessor: this.summary(other),
          current_successors: this.currentSuccessors(other).map((tip) => this.summary(tip)),
        })),
      diagnostics: [],
    };
  }

  /** Accept a set of candidates; each predecessor gains every candidate in the set that names it. */
  accept(queries: string[]): MockRecord[] {
    const set = queries.map((query) => this.get(query));
    for (const candidate of set) {
      if (candidate.status !== 'candidate') throw new Error(`\`${candidate.title}\` is not a candidate`);
    }
    const ids = new Set(set.map((candidate) => candidate.id));
    const edges = new Map<MockRecord, MockRecord[]>();
    for (const candidate of set) {
      for (const predecessor of this.named(candidate)) {
        if (ids.has(predecessor.id)) {
          throw new Error(`\`${candidate.title}\` names \`${predecessor.title}\`, which is accepted in the same set`);
        }
        if (predecessor.status !== 'accepted') {
          throw new Error(`\`${predecessor.title}\` is ${predecessor.status}; a successor can supersede only an accepted decision`);
        }
        if (this.isSuperseded(predecessor)) {
          const tips = this.currentSuccessors(predecessor).map((tip) => `\`${tip.title}\``).join(', ');
          throw new Error(`\`${predecessor.title}\` is already superseded by ${tips}; retarget \`## Supersedes\` at the current decision`);
        }
        edges.set(predecessor, [...(edges.get(predecessor) ?? []), candidate]);
      }
    }
    for (const [predecessor, successors] of edges) {
      predecessor.successors.push(...successors.map((successor) => successor.id));
      predecessor.raw = undefined;
    }
    for (const candidate of set) {
      candidate.status = 'accepted';
      candidate.raw = undefined;
      // As if the acceptance were committed: later edits show as text edits.
      candidate.head = { title: candidate.title, body: candidate.body };
      candidate.log = [];
    }
    return set;
  }

  reject(query: string): MockRecord {
    const record = this.get(query);
    if (record.status !== 'candidate') throw new Error(`\`${record.title}\` is not a candidate`);
    record.status = 'rejected';
    record.raw = undefined;
    record.head = { title: record.title, body: record.body };
    record.log = [];
    return record;
  }

  /**
   * The edits to a record's title or body since it was accepted or rejected,
   * newest first: the uncommitted working-tree edit, then each commit. Null
   * outside Git; candidates have none.
   */
  edits(record: MockRecord, patch: boolean): RecordEdit[] | null {
    if (!this.git) return null;
    if (record.status === 'candidate') return [];
    const edits: RecordEdit[] = [];
    const working = { title: record.title, body: record.body };
    if (record.head && textChanged(record.head, working)) {
      edits.push(this.edit(null, record.head, working, record.tags, patch));
    }
    for (const commit of record.log ?? []) {
      edits.push(this.edit(commit, commit.before, commit.after, record.tags, patch));
    }
    return edits;
  }

  private edit(commit: MockCommit | null, before: MockText, after: MockText, tags: string[], patch: boolean): RecordEdit {
    const edit: RecordEdit = {
      commit: commit?.commit ?? null,
      short_commit: commit?.commit.slice(0, 7) ?? null,
      author: commit?.author ?? null,
      date: commit?.date ?? null,
      subject: commit?.subject ?? null,
      title_changed: before.title !== after.title,
      body_changed: comparableBody(before.body) !== comparableBody(after.body),
    };
    if (patch) edit.diff = this.diff({ ...before, tags }, { ...after, tags });
    return edit;
  }

  historyRecord(record: MockRecord): HistoryRecord {
    return {
      id: record.id,
      short_id: record.id.replaceAll('-', '').slice(0, 12),
      title: record.title,
      status: record.status,
      superseded: this.isSuperseded(record),
      current: this.liveSuccessors(record).length === 0,
      path: this.path(record),
      tags: [...record.tags],
    };
  }

  /** The history of the decision containing `query`, as `dadrock history --json` returns it. */
  history(query: string, patch: boolean): RecordHistory {
    const opened = this.get(query);
    const versions = new Set<MockRecord>([opened]);
    const walk = (start: MockRecord, next: (record: MockRecord) => MockRecord[]) => {
      const queue = next(start);
      while (queue.length) {
        const record = queue.shift() as MockRecord;
        if (versions.has(record)) continue;
        versions.add(record);
        queue.push(...next(record));
      }
    };
    walk(opened, (record) => this.predecessors(record));
    walk(opened, (record) => this.liveSuccessors(record));

    // Newest first, every record above its predecessors; ties by id.
    const order: MockRecord[] = [];
    const remaining = new Map(
      [...versions].map((record) => [record, this.liveSuccessors(record).filter((other) => versions.has(other)).length]),
    );
    while (remaining.size) {
      const ready = [...remaining].filter(([, count]) => count === 0).map(([record]) => record);
      const pool = ready.length ? ready : [...remaining.keys()];
      pool.sort((left, right) => right.timestamp_ms - left.timestamp_ms || right.id.localeCompare(left.id));
      const next = pool[0] as MockRecord;
      order.push(next);
      remaining.delete(next);
      for (const predecessor of this.predecessors(next)) {
        const count = remaining.get(predecessor);
        if (count !== undefined) remaining.set(predecessor, count - 1);
      }
    }

    const edges: Array<[MockRecord, MockRecord]> = [];
    const seen = new Set<string>();
    for (const record of order) {
      const touching: Array<[MockRecord, MockRecord]> = [
        ...this.liveSuccessors(record).map((successor): [MockRecord, MockRecord] => [record, successor]),
        ...this.predecessors(record).map((predecessor): [MockRecord, MockRecord] => [predecessor, record]),
      ];
      for (const [predecessor, successor] of touching) {
        const key = `${predecessor.id}>${successor.id}`;
        if (seen.has(key)) continue;
        seen.add(key);
        edges.push([predecessor, successor]);
      }
    }
    const neighbors = [...new Set(edges.flat())]
      .filter((record) => !versions.has(record))
      .sort((left, right) => right.id.localeCompare(left.id));
    const rank = new Map([...order, ...neighbors].map((record, index) => [record, index]));
    edges.sort(
      ([leftPredecessor, leftSuccessor], [rightPredecessor, rightSuccessor]) =>
        (rank.get(leftSuccessor) ?? 0) - (rank.get(rightSuccessor) ?? 0) ||
        (rank.get(leftPredecessor) ?? 0) - (rank.get(rightPredecessor) ?? 0),
    );

    const links: HistoryLink[] = edges.map(([predecessor, successor]) => {
      const section = supersedesSection(successor.body);
      const names = section?.links.some((link) => link.id.toLowerCase() === predecessor.id);
      const link: HistoryLink = {
        predecessor: predecessor.id,
        successor: successor.id,
        reason: names ? section?.reason ?? null : null,
      };
      if (patch) link.diff = this.diff(predecessor, successor);
      return link;
    });

    return {
      record: opened.id,
      versions: order.map((record) => ({
        ...this.historyRecord(record),
        body: comparableBody(record.body),
        edits: this.edits(record, patch),
      })),
      links,
      neighbors: neighbors.map((record) => this.historyRecord(record)),
    };
  }

  /** The difference from `old` to `new`, as `dadrock diff` computes it. */
  diff(old: DiffSide, next: DiffSide): RecordDiff {
    const title = old.title !== next.title ? { old: old.title, new: next.title } : null;
    const key = (tag: string) => tag.trim().replace(/^#+/, '').toLowerCase();
    const missing = (tags: string[], other: string[]) => tags.filter((tag) => !other.map(key).includes(key(tag)));
    const tags_added = missing(next.tags, old.tags);
    const tags_removed = missing(old.tags, next.tags);
    const lines = lineDiff(comparableBody(old.body), comparableBody(next.body)).map((line) => ({
      kind: line.kind === 'same' ? ('context' as const) : line.kind === 'add' ? ('added' as const) : ('removed' as const),
      text: line.text,
    }));
    return {
      changed: Boolean(title) || tags_added.length > 0 || tags_removed.length > 0 || lines.some((line) => line.kind !== 'context'),
      title,
      tags_added,
      tags_removed,
      lines,
    };
  }

  diffResult(oldQuery: string, newQuery: string): RecordDiffResult {
    const old = this.get(oldQuery);
    const next = this.get(newQuery);
    return { old: this.historyRecord(old), new: this.historyRecord(next), diff: this.diff(old, next) };
  }
}

type DiffSide = Pick<MockRecord, 'title' | 'tags' | 'body'>;

/** Whether Git would list a text edit: the title, or the body without `## Supersedes`, differs. */
function textChanged(before: MockText, after: MockText): boolean {
  return before.title !== after.title || comparableBody(before.body) !== comparableBody(after.body);
}

function sameMembers(left: string[], right: string[]): boolean {
  const key = (ids: string[]) => [...new Set(ids.map((id) => id.toLowerCase()))].sort().join(',');
  return key(left) === key(right);
}
