import { describe, expect, it } from 'vitest';
import type { HistoryLink, HistoryRecord, RecordHistory } from '../../src/lib/api';
import { currentVersions, historyLayout } from '../../src/lib/history/layout';

function id(n: number): string {
  return `0193fb1f-a3a0-7000-8000-${String(n).padStart(12, '0')}`;
}

function summary(n: number, title: string, current = false): HistoryRecord {
  return {
    id: id(n),
    short_id: id(n).replaceAll('-', '').slice(0, 12),
    title,
    status: 'accepted',
    superseded: !current,
    current,
    path: `dadr/x/${n}.md`,
    tags: [],
  };
}

function version(n: number, title: string, current = false) {
  return { ...summary(n, title, current), body: `${title}\n`, edits: null };
}

function link(predecessor: number, successor: number, reason: string | null = null): HistoryLink {
  return { predecessor: id(predecessor), successor: id(successor), reason };
}

// Pairwise (1) -> Listwise (2) -> split into Multiple imputation (4) and Complete cases (3).
const P = 1;
const L = 2;
const C = 3;
const M = 4;

describe('historyLayout', () => {
  it('draws a chain in one lane, newest first', () => {
    const history: RecordHistory = {
      record: id(L),
      versions: [version(L, 'Listwise', true), version(P, 'Pairwise')],
      links: [link(P, L, 'Complete cases are simpler.')],
      neighbors: [],
    };
    const layout = historyLayout(history);
    expect(layout.lanes).toBe(1);
    expect(layout.rows.map((row) => [row.version.title, row.lane, row.fromAbove, row.toBelow, row.opened])).toEqual([
      ['Listwise', 0, false, true, true],
      ['Pairwise', 0, true, false, false],
    ]);
    expect(layout.rows[0]?.reason).toBe('Complete cases are simpler.');
    expect(layout.rows[0]?.supersedes.map((entry) => entry.predecessor.title)).toEqual(['Pairwise']);
    expect(layout.rows[1]?.supersedes).toEqual([]);
  });

  it('joins the lanes of a split at the shared predecessor', () => {
    const history: RecordHistory = {
      record: id(L),
      versions: [version(M, 'Multiple imputation', true), version(C, 'Complete cases', true), version(L, 'Listwise'), version(P, 'Pairwise')],
      links: [link(L, M, 'Different treatment.'), link(L, C, 'Different treatment.'), link(P, L)],
      neighbors: [],
    };
    const layout = historyLayout(history);
    expect(layout.lanes).toBe(2);
    expect(layout.rows.map(({ lane, through, joins, forks, fromAbove, toBelow }) => ({ lane, through, joins, forks, fromAbove, toBelow }))).toEqual([
      { lane: 0, through: [], joins: [], forks: [], fromAbove: false, toBelow: true },
      { lane: 1, through: [0], joins: [], forks: [], fromAbove: false, toBelow: true },
      { lane: 0, through: [], joins: [1], forks: [], fromAbove: true, toBelow: true },
      { lane: 0, through: [], joins: [], forks: [], fromAbove: true, toBelow: false },
    ]);
    expect(layout.rows[2]?.reason).toBeNull();
    expect(currentVersions(history).map((entry) => entry.title)).toEqual(['Multiple imputation', 'Complete cases']);
  });

  it('forks the lanes of a merge from the successor', () => {
    // Trim (5) -> Winsorize (7); Winsorize and Ages (6) merge into One rule (8).
    const history: RecordHistory = {
      record: id(8),
      versions: [version(8, 'One rule', true), version(7, 'Winsorize'), version(6, 'Ages'), version(5, 'Trim')],
      links: [link(7, 8, 'They disagreed.'), link(6, 8, 'They disagreed.'), link(5, 7)],
      neighbors: [],
    };
    const layout = historyLayout(history);
    expect(layout.lanes).toBe(2);
    expect(layout.rows.map(({ lane, through, joins, forks, fromAbove, toBelow }) => ({ lane, through, joins, forks, fromAbove, toBelow }))).toEqual([
      { lane: 0, through: [], joins: [], forks: [1], fromAbove: false, toBelow: true },
      { lane: 0, through: [1], joins: [], forks: [], fromAbove: true, toBelow: true },
      { lane: 1, through: [0], joins: [], forks: [], fromAbove: true, toBelow: false },
      { lane: 0, through: [], joins: [], forks: [], fromAbove: true, toBelow: false },
    ]);
    expect(layout.rows[0]?.supersedes.map((entry) => entry.predecessor.title)).toEqual(['Winsorize', 'Ages']);
  });

  it('lists the other half of a split and the other input of a merge as neighbors', () => {
    const split: RecordHistory = {
      record: id(C),
      versions: [version(C, 'Complete cases', true), version(L, 'Listwise'), version(P, 'Pairwise')],
      links: [link(L, M, 'Different treatment.'), link(L, C, 'Different treatment.'), link(P, L)],
      neighbors: [summary(M, 'Multiple imputation', true)],
    };
    const splitRows = historyLayout(split).rows;
    expect(splitRows[0]?.reason).toBe('Different treatment.');
    expect(splitRows[1]?.alsoBecame.map((neighbor) => neighbor.title)).toEqual(['Multiple imputation']);
    expect(historyLayout(split).lanes).toBe(1);

    const merge: RecordHistory = {
      record: id(7),
      versions: [version(8, 'One rule', true), version(7, 'Winsorize'), version(5, 'Trim')],
      links: [link(7, 8, 'They disagreed.'), link(6, 8, 'They disagreed.'), link(5, 7)],
      neighbors: [summary(6, 'Ages')],
    };
    const mergeRows = historyLayout(merge).rows;
    expect(mergeRows[0]?.mergedWith.map((neighbor) => neighbor.title)).toEqual(['Ages']);
    expect(mergeRows[0]?.supersedes.map((entry) => entry.predecessor.title)).toEqual(['Winsorize']);
    expect(mergeRows[1]?.opened).toBe(true);
  });
});
