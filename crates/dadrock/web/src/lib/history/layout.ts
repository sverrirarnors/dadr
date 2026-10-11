import type { HistoryLink, HistoryRecord, HistoryVersion, RecordHistory } from '$lib/api';

/**
 * One row of the History view: a version, where its node sits among the
 * lanes, and the lines drawn around it. Lanes are numbered from the left.
 */
export interface HistoryRow {
  version: HistoryVersion;
  /** The record the history was opened at. */
  opened: boolean;
  /** The lane of this version's node. */
  lane: number;
  /** A line comes into the node from a successor above. */
  fromAbove: boolean;
  /** A line leaves the node toward a predecessor below. */
  toBelow: boolean;
  /** Lanes drawn straight through the row. */
  through: number[];
  /** Lanes that end at this node: several successors share this predecessor (a split). */
  joins: number[];
  /** Lanes that start at this node: this version has several predecessors (a merge). */
  forks: number[];
  /** Links from a predecessor that is a version, with their diffs. */
  supersedes: Array<{ link: HistoryLink; predecessor: HistoryVersion }>;
  /** Why this version took the place of its predecessors, from its `## Supersedes`. */
  reason: string | null;
  /** Other successors of this version that are not versions: the other half of a split. */
  alsoBecame: HistoryRecord[];
  /** Other predecessors of this version that are not versions: the other input of a merge. */
  mergedWith: HistoryRecord[];
}

export interface HistoryLayout {
  rows: HistoryRow[];
  /** How many lanes the widest row needs. */
  lanes: number;
}

/**
 * Lay a history out like `git log --graph`: versions newest first, each in a
 * lane that runs down to its predecessor. A split shows as lanes joining at
 * the shared predecessor; a merge as lanes forking from the successor.
 */
export function historyLayout(history: RecordHistory): HistoryLayout {
  const versions = new Map(history.versions.map((version) => [version.id, version]));
  const neighbors = new Map(history.neighbors.map((neighbor) => [neighbor.id, neighbor]));
  const order = new Map(history.versions.map((version, index) => [version.id, index]));

  const lanes: Array<string | null> = [];
  let width = 0;
  const rows: HistoryRow[] = [];

  for (const version of history.versions) {
    const incoming = history.links.filter((link) => link.successor === version.id);
    const predecessorIds = incoming
      .map((link) => link.predecessor)
      .filter((id) => versions.has(id))
      .sort((left, right) => (order.get(left) ?? 0) - (order.get(right) ?? 0));

    const waiting = lanes.flatMap((expected, index) => (expected === version.id ? [index] : []));
    const active = lanes.flatMap((expected, index) => (expected ? [index] : []));
    const lane = waiting[0] ?? freeLane(lanes, []);
    const joins = waiting.slice(1);
    const through = active.filter((index) => index !== lane && !joins.includes(index));

    for (const index of [lane, ...joins]) lanes[index] = null;
    const forks: number[] = [];
    predecessorIds.forEach((predecessorId, index) => {
      if (index === 0) {
        lanes[lane] = predecessorId;
        return;
      }
      const existing = lanes.indexOf(predecessorId);
      if (existing >= 0) {
        forks.push(existing);
        return;
      }
      const next = freeLane(lanes, joins);
      lanes[next] = predecessorId;
      forks.push(next);
    });
    width = Math.max(width, lanes.length, lane + 1);

    rows.push({
      version,
      opened: version.id === history.record,
      lane,
      fromAbove: waiting.length > 0,
      toBelow: predecessorIds.length > 0,
      through,
      joins,
      forks,
      supersedes: predecessorIds.map((id) => ({
        link: incoming.find((link) => link.predecessor === id) as HistoryLink,
        predecessor: versions.get(id) as HistoryVersion,
      })),
      reason: incoming.find((link) => link.reason)?.reason ?? null,
      alsoBecame: history.links
        .filter((link) => link.predecessor === version.id && neighbors.has(link.successor))
        .map((link) => neighbors.get(link.successor) as HistoryRecord),
      mergedWith: incoming
        .filter((link) => neighbors.has(link.predecessor))
        .map((link) => neighbors.get(link.predecessor) as HistoryRecord),
    });
  }

  return { rows, lanes: Math.max(width, 1) };
}

/** The current versions: what the decision is now. */
export function currentVersions(history: RecordHistory): HistoryVersion[] {
  return history.versions.filter((version) => version.current);
}

function freeLane(lanes: Array<string | null>, avoid: number[]): number {
  const index = lanes.findIndex((expected, lane) => expected === null && !avoid.includes(lane));
  if (index >= 0) return index;
  lanes.push(null);
  return lanes.length - 1;
}
