import { describe, expect, it } from 'vitest';
import { retargetSource } from '../../src/lib/records/retarget';
import {
  bodyWithoutSupersedes,
  comparableBody,
  retargetSupersedes,
  supersededIds,
  supersedesLink,
  supersedesSection,
} from '../../src/lib/records/supersedes';

const OLD = '01938c20-a3b4-7000-8000-111122223333';
const AGES = '0193a0b7-e911-7000-8000-444455556666';
const NEW = '0193b2a1-c0f4-7000-8000-777788889999';
const OTHER = '0193b2a1-c0f4-7000-8000-aaaabbbbcccc';

const merge = [
  '## Supersedes',
  '',
  `[[${OLD}|Winsorize income at p99]]`,
  `[[${AGES}|Drop implausible ages]]`,
  '',
  'The two rules disagreed at the 99th percentile;',
  'one rule now covers every continuous covariate.',
  '',
  '## Context',
  '',
  'Outliers.',
  '',
].join('\n');

describe('supersedesSection', () => {
  it('reads the links and the reason', () => {
    expect(supersedesSection(merge)).toEqual({
      links: [
        { id: OLD, label: 'Winsorize income at p99' },
        { id: AGES, label: 'Drop implausible ages' },
      ],
      reason: 'The two rules disagreed at the 99th percentile;\none rule now covers every continuous covariate.',
    });
    expect(supersededIds(merge)).toEqual([OLD, AGES]);
  });

  it('accepts links without labels and sections without a reason', () => {
    expect(supersedesSection(`## Supersedes\n\n[[${OLD}]]\n`)).toEqual({ links: [{ id: OLD, label: null }], reason: null });
  });

  it('is null without a section, without links, or with a link that is not a full id', () => {
    expect(supersedesSection('## Context\n\nNothing.\n')).toBeNull();
    expect(supersedesSection('## Supersedes\n\nJust a reason.\n')).toBeNull();
    expect(supersedesSection('## Supersedes\n\n[[01938c20a3b4|Short id]]\n')).toBeNull();
    expect(supersedesSection(`## Supersedes\n\n[[${OLD}]]\n[[${OLD}]]\n`)).toBeNull();
    expect(supersedesSection(`## Supersedes\n\n[[${OLD}]]\n\n## Supersedes\n\n[[${AGES}]]\n`)).toBeNull();
  });

  it('ignores headings and links inside code', () => {
    const body = ['```', '## Supersedes', `[[${OLD}]]`, '```', '', '## Context', '', 'x'].join('\n');
    expect(supersedesSection(body)).toBeNull();
    const indented = `## Supersedes\n\n    [[${OLD}]]\n`;
    expect(supersedesSection(indented)).toBeNull();
  });
});

describe('bodyWithoutSupersedes', () => {
  it('removes the section and keeps the rest as written', () => {
    expect(bodyWithoutSupersedes(merge)).toBe('## Context\n\nOutliers.\n');
    expect(bodyWithoutSupersedes('## Context\n\nOutliers.\n')).toBe('## Context\n\nOutliers.\n');
  });

  it('drops blank lines left at the end by a final section', () => {
    expect(bodyWithoutSupersedes(`## Context\n\nx\n\n## Supersedes\n\n[[${OLD}]]\n\nWhy.\n`)).toBe('## Context\n\nx\n');
  });
});

describe('comparableBody', () => {
  it('normalizes line endings and trims blank lines around the text', () => {
    expect(comparableBody(`\r\n${merge.replace(/\n/g, '\r\n')}\r\n\r\n`)).toBe('## Context\n\nOutliers.\n');
    expect(comparableBody('\n\n')).toBe('');
  });
});

describe('retargetSupersedes', () => {
  it('points the stale link at the current decision and keeps everything else', () => {
    const rewritten = retargetSupersedes(merge, OLD, [{ id: NEW, title: 'Multiple imputation' }]);
    expect(rewritten).toBe(merge.replace(`[[${OLD}|Winsorize income at p99]]`, `[[${NEW}|Multiple imputation]]`));
    expect(supersededIds(rewritten ?? '')).toEqual([NEW, AGES]);
    expect(supersedesSection(rewritten ?? '')?.reason).toBe(supersedesSection(merge)?.reason);
  });

  it('writes one link per target and skips decisions the section already names', () => {
    const rewritten = retargetSupersedes(merge, OLD, [
      { id: NEW, title: 'Half one' },
      { id: AGES, title: 'Drop implausible ages' },
      { id: OTHER, title: 'Half [[two]]' },
    ]);
    expect(supersededIds(rewritten ?? '')).toEqual([NEW, OTHER, AGES]);
    expect(rewritten).toContain(`[[${OTHER}|Half [two]]]`);
  });

  it('keeps CRLF line endings', () => {
    const crlf = merge.replace(/\n/g, '\r\n');
    expect(retargetSupersedes(crlf, AGES, [{ id: NEW, title: 'New' }])).toBe(
      crlf.replace(`[[${AGES}|Drop implausible ages]]`, `[[${NEW}|New]]`),
    );
  });

  it('is null when the section does not link the record', () => {
    expect(retargetSupersedes(merge, NEW, [{ id: OTHER, title: 'x' }])).toBeNull();
    expect(retargetSupersedes('## Context\n', OLD, [{ id: NEW, title: 'x' }])).toBeNull();
  });
});

describe('retargetSource', () => {
  it('rewrites only the body of a record file', () => {
    const source = `---\ntitle: Merge\nstatus: candidate\nid: ${NEW}\n---\n\n${merge}`;
    const rewritten = retargetSource(source, AGES, [{ id: OTHER, title: 'Cap ages' }]);
    expect(rewritten.startsWith(`---\ntitle: Merge\nstatus: candidate\nid: ${NEW}\n---\n\n## Supersedes\n`)).toBe(true);
    expect(supersededIds(rewritten.slice(rewritten.indexOf('## Supersedes')))).toEqual([OLD, OTHER]);
    expect(() => retargetSource(source, OTHER, [{ id: NEW, title: 'x' }])).toThrow(/does not link/);
    expect(() => retargetSource(source, OLD, [])).toThrow(/Choose/);
  });
});

describe('supersedesLink', () => {
  it('labels the link with a one-line title', () => {
    expect(supersedesLink({ id: OLD, title: ' Two\nlines ' })).toBe(`[[${OLD}|Two lines]]`);
    expect(supersedesLink({ id: OLD, title: '' })).toBe(`[[${OLD}]]`);
  });
});
