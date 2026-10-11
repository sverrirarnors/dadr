import { describe, expect, it } from 'vitest';
import { bodyPreamble, bodySections, compareSections, mergedSuccessorBody } from '../../src/lib/records/sections';

const trim = [
  '## Context',
  '',
  'Income has a long tail.',
  '',
  '## Decision',
  '',
  'Trim income at p95.',
  '',
  '```r',
  '## not a heading',
  '```',
  '',
  '## Alternatives',
  '',
  'Keep everything.',
  '',
  '## Consequences',
  '',
  'Five percent dropped.',
].join('\n');

const cap = '## Context\n\nAges.\n\n## Decision\n\nCap ages at 99.\n\n## Notes\n\nSee codebook.\n';
const merged = '## Supersedes\n\n[[01938c20-a3b4-7000-8000-111122223333]]\n\nOne rule.\n\n## Decision\n\nWinsorize at p99.\n\n## Context\n\nTails.\n';

describe('bodySections', () => {
  it('splits level-2 sections and ignores headings in code', () => {
    const sections = bodySections(trim);
    expect([...sections.keys()]).toEqual(['Context', 'Decision', 'Alternatives', 'Consequences']);
    expect(sections.get('Decision')).toBe('Trim income at p95.\n\n```r\n## not a heading\n```');
  });
});

describe('compareSections', () => {
  it('puts Decision first, then each predecessor followed by the new text', () => {
    const comparisons = compareSections(
      [
        { id: 'a', title: 'Trim income at p95', body: trim },
        { id: 'b', title: 'Cap ages at 99', body: cap },
      ],
      merged,
    );
    expect(comparisons.map((comparison) => comparison.section)).toEqual([
      'Decision',
      'Context',
      'Alternatives',
      'Consequences',
    ]);
    expect(comparisons[0]).toEqual({
      section: 'Decision',
      predecessors: [
        { id: 'a', title: 'Trim income at p95', text: 'Trim income at p95.\n\n```r\n## not a heading\n```' },
        { id: 'b', title: 'Cap ages at 99', text: 'Cap ages at 99.' },
      ],
      successor: 'Winsorize at p99.',
    });
    expect(comparisons[2]?.predecessors[1]?.text).toBeNull();
    expect(comparisons[2]?.successor).toBeNull();
  });
});

describe('mergedSuccessorBody', () => {
  it('starts a merge with every predecessor text per section, required sections first', () => {
    expect(mergedSuccessorBody([trim, cap])).toBe(
      [
        '## Context',
        '',
        'Income has a long tail.',
        '',
        'Ages.',
        '',
        '## Decision',
        '',
        'Trim income at p95.',
        '',
        '```r',
        '## not a heading',
        '```',
        '',
        'Cap ages at 99.',
        '',
        '## Alternatives',
        '',
        'Keep everything.',
        '',
        '## Consequences',
        '',
        'Five percent dropped.',
        '',
        '## Notes',
        '',
        'See codebook.',
        '',
      ].join('\n'),
    );
  });
});

describe('bodyPreamble', () => {
  it('keeps text that comes before any section, also in a merge', () => {
    expect(bodyPreamble('\nUse lazy frames.\n\n## Context\n\nx\n')).toBe('Use lazy frames.');
    expect(bodyPreamble('## Context\n\nx\n')).toBe('');
    expect(mergedSuccessorBody(['Use lazy frames.\n', '## Decision\n\nUse Arrow.\n'])).toBe(
      'Use lazy frames.\n\n## Context\n\n## Decision\n\nUse Arrow.\n\n## Alternatives\n\n## Consequences\n',
    );
  });
});
