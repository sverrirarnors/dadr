import { describe, expect, it } from 'vitest';
import { diffSides, diffStats, fromRecordDiffLines, hasChanges, lineDiff, tagChangeSummary } from '../../src/lib/inbox/diff';

describe('lineDiff', () => {
  it('marks identical documents as unchanged', () => {
    const lines = lineDiff('a\nb\nc', 'a\nb\nc');
    expect(lines).toEqual([
      { kind: 'same', text: 'a' },
      { kind: 'same', text: 'b' },
      { kind: 'same', text: 'c' },
    ]);
    expect(hasChanges(lines)).toBe(false);
  });

  it('finds inserted and removed lines around a common core', () => {
    const lines = lineDiff('## Context\nold\n## Decision\nkeep', '## Context\nnew\n## Decision\nkeep\nmore');
    expect(lines).toEqual([
      { kind: 'same', text: '## Context' },
      { kind: 'remove', text: 'old' },
      { kind: 'add', text: 'new' },
      { kind: 'same', text: '## Decision' },
      { kind: 'same', text: 'keep' },
      { kind: 'add', text: 'more' },
    ]);
    expect(diffStats(lines)).toEqual({ added: 2, removed: 1 });
  });

  it('treats an empty side as all additions or all removals', () => {
    expect(lineDiff('', 'x\ny')).toEqual([
      { kind: 'add', text: 'x' },
      { kind: 'add', text: 'y' },
    ]);
    expect(lineDiff('x', '')).toEqual([{ kind: 'remove', text: 'x' }]);
    expect(lineDiff('', '')).toEqual([]);
  });

  it('ignores trailing newlines and CRLF differences', () => {
    expect(hasChanges(lineDiff('a\r\nb\r\n', 'a\nb'))).toBe(false);
    expect(lineDiff('a\n\n', 'a\n')).toEqual([
      { kind: 'same', text: 'a' },
      { kind: 'remove', text: '' },
    ]);
  });

  it('keeps the longest common subsequence when lines move', () => {
    const lines = lineDiff('a\nb\nc\nd', 'b\nc\nd\na');
    expect(lines.filter((line) => line.kind === 'same').map((line) => line.text)).toEqual(['b', 'c', 'd']);
    expect(diffStats(lines)).toEqual({ added: 1, removed: 1 });
  });
});

describe('record diff lines', () => {
  it('maps Dadrock diff kinds and rebuilds both texts', () => {
    const lines = fromRecordDiffLines([
      { kind: 'context', text: '## Decision' },
      { kind: 'removed', text: 'Drop rows.' },
      { kind: 'added', text: 'Impute rows.' },
    ]);
    expect(lines).toEqual([
      { kind: 'same', text: '## Decision' },
      { kind: 'remove', text: 'Drop rows.' },
      { kind: 'add', text: 'Impute rows.' },
    ]);
    expect(diffSides(lines)).toEqual({ before: '## Decision\nDrop rows.\n', after: '## Decision\nImpute rows.\n' });
  });

  it('summarizes tag changes', () => {
    expect(tagChangeSummary(['topic/missing-data'], ['topic/attrition'])).toBe('+topic/missing-data −topic/attrition');
    expect(tagChangeSummary([], [])).toBe('');
  });
});
