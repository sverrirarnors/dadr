import { describe, expect, it } from 'vitest';
import {
  buildTagTree,
  flattenTagTree,
  formatTagCount,
  tagRecordsHref,
} from '../../src/lib/tags/tree';

const tags = [
  { tag: '#dadr/data/processing', display: 'dadr/data/processing', record_count: 2, occurrence_count: 3 },
  { tag: '#dadr/data/visualization', display: 'dadr/data/visualization', record_count: 1, occurrence_count: 1 },
  { tag: '#dadr/methodology', display: 'dadr/methodology', record_count: 1, occurrence_count: 2 },
  { tag: '#team', display: 'team', record_count: 3, occurrence_count: 3 },
];

describe('buildTagTree', () => {
  it('groups slash-delimited tags into a sorted aggregate tree', () => {
    const tree = buildTagTree(tags);

    expect(tree.map((node) => node.pathTag)).toEqual(['#dadr', '#team']);
    expect(tree[0]).toMatchObject({
      label: 'dadr',
      pathTag: '#dadr',
      directRecordCount: 0,
      aggregateRecordCount: 4,
      aggregateOccurrenceCount: 6,
    });
    expect(tree[0]?.children.map((node) => node.pathTag)).toEqual(['#dadr/data', '#dadr/methodology']);
    expect(tree[0]?.children[0]).toMatchObject({
      label: 'data',
      pathTag: '#dadr/data',
      directRecordCount: 0,
      aggregateRecordCount: 3,
    });
    expect(tree[0]?.children[0]?.children.map((node) => node.pathTag)).toEqual([
      '#dadr/data/processing',
      '#dadr/data/visualization',
    ]);
  });
});

describe('flattenTagTree', () => {
  it('returns display rows with depth for indentation', () => {
    const rows = flattenTagTree(buildTagTree(tags));

    expect(rows.map((row) => [row.depth, row.pathTag])).toEqual([
      [0, '#dadr'],
      [1, '#dadr/data'],
      [2, '#dadr/data/processing'],
      [2, '#dadr/data/visualization'],
      [1, '#dadr/methodology'],
      [0, '#team'],
    ]);
  });
});

describe('tagRecordsHref', () => {
  it('builds a records search URL for a tag path', () => {
    expect(tagRecordsHref('/records', '#dadr/data/processing')).toBe(
      '/records?q=%23dadr%2Fdata%2Fprocessing',
    );
  });
});

describe('formatTagCount', () => {
  it('shows direct and aggregate counts when they differ', () => {
    const tree = buildTagTree(tags);

    expect(formatTagCount(tree[0]!)).toBe('4 records, 6 uses');
    expect(formatTagCount(tree[0]!.children[0]!)).toBe('3 records, 4 uses');
    expect(formatTagCount(tree[0]!.children[0]!.children[0]!)).toBe('2 records, 3 uses');
    expect(formatTagCount(tree[1]!)).toBe('3 records');
  });
});
