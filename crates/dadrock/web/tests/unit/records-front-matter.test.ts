import { describe, expect, it } from 'vitest';
import { readFrontMatter, splitSource, yamlScalar } from '../../src/lib/records/front-matter';

const source = [
  '---',
  'title: Use cluster-robust standard errors',
  'status: accepted',
  'id: 01938b94-bcd0-7e00-8b00-000000000006',
  'tags:',
  '  - kind/inference',
  '  - "#dadr/methodology"',
  'bibliography:',
  '  - ../bibliography.bib',
  '# a comment',
  '---',
  '',
  '## Context',
  '',
  'Sites.',
  '',
].join('\n');

describe('splitSource', () => {
  it('splits after the closing delimiter and joins back to the source', () => {
    const parts = splitSource(source);
    expect(parts.frontMatter.endsWith('# a comment\n---\n')).toBe(true);
    expect(parts.body).toBe('\n## Context\n\nSites.\n');
    expect(parts.frontMatter + parts.body).toBe(source);
  });

  it('treats a file without front matter as all body', () => {
    expect(splitSource('## Context\n')).toEqual({ frontMatter: '', body: '## Context\n' });
  });
});

describe('readFrontMatter', () => {
  it('reads the DADR keys, block and flow lists, and quoted values', () => {
    expect(readFrontMatter(source)).toEqual({
      title: 'Use cluster-robust standard errors',
      status: 'accepted',
      id: '01938b94-bcd0-7e00-8b00-000000000006',
      tags: ['kind/inference', '#dadr/methodology'],
      successors: [],
    });
    expect(
      readFrontMatter("---\ntitle: 'It''s fine'\nstatus: candidate\nid: x\ntags: [a, \"b, c\"]\nsuccessors: []\n---\n"),
    ).toMatchObject({ title: "It's fine", tags: ['a', 'b, c'], successors: [] });
  });

  it('reads block lists written without indentation, as Dadrock writes them', () => {
    const dadrock = '---\ntitle: T\nstatus: accepted\nid: x\ntags:\n- kind/input\n- topic/assignment\nsuccessors:\n- a\n---\n\nBody\n';
    expect(readFrontMatter(dadrock)).toMatchObject({ tags: ['kind/input', 'topic/assignment'], successors: ['a'] });
  });
});

describe('yamlScalar', () => {
  it('quotes values that YAML would read differently', () => {
    expect(yamlScalar('kind/sample')).toBe('kind/sample');
    expect(yamlScalar('#dadr/x')).toBe('"#dadr/x"');
    expect(yamlScalar('a: b')).toBe('"a: b"');
  });
});
