import { describe, expect, it } from 'vitest';
import type { RecordSummary } from '../../src/lib/api';
import {
  attachedRecords,
  attachRecordsStatus,
  composerTitle,
  errorMessage,
  initialComposerView,
  ledgerEditHref,
  manualFormAvailable,
  pinnedRecord,
  promptLabel,
  successorChanged,
  successorPrefill,
  successorStartingBody,
} from '../../src/lib/draft/composer-view';

function record(
  id: string,
  status: RecordSummary['status'],
  title: string,
  extra: Partial<RecordSummary> = {},
): RecordSummary {
  return {
    id,
    id_prefix: id.slice(0, 12),
    simple_record_id: id,
    title,
    status,
    superseded: false,
    path: `dadr/${status}/${id}.md`,
    preview: '',
    tags: [],
    timestamp_ms: 0,
    ...extra,
  };
}

const accepted = record('acc-1', 'accepted', 'Use Polars', { tags: ['#dadr/data', '#kind/setup'] });
const candidate = record('cand-1', 'candidate', 'First draft');
const superseded = record('acc-0', 'accepted', 'Use pandas', { superseded: true });
const records = [accepted, candidate, superseded];

describe('pinnedRecord', () => {
  it('returns the first attached record for revise and supersede', () => {
    expect(pinnedRecord('revise', ['cand-1'], records)).toBe(candidate);
    expect(pinnedRecord('supersede', ['acc-1', 'cand-1'], records)).toBe(accepted);
  });

  it('is undefined for other modes, missing ids, or unknown records', () => {
    expect(pinnedRecord('new', ['acc-1'], records)).toBeUndefined();
    expect(pinnedRecord('merge', ['cand-1'], records)).toBeUndefined();
    expect(pinnedRecord('revise', [], records)).toBeUndefined();
    expect(pinnedRecord('revise', ['nope'], records)).toBeUndefined();
  });
});

describe('composerTitle', () => {
  it('names the verb and the record', () => {
    expect(composerTitle('new', [])).toBe('Draft a decision');
    expect(composerTitle('merge', [])).toBe('Merge candidates');
    expect(composerTitle('revise', [candidate])).toBe('Revise “First draft”');
    expect(composerTitle('supersede', [accepted])).toBe('Supersede “Use Polars”');
  });

  it('calls several decisions superseded together a merge', () => {
    expect(composerTitle('supersede', [accepted, superseded])).toBe('Supersede 2 decisions');
  });

  it('falls back when the record is unknown', () => {
    expect(composerTitle('revise', [])).toBe('Revise a record');
    expect(composerTitle('supersede', [])).toBe('Supersede a decision');
  });
});

describe('attachedRecords', () => {
  it('keeps the attachment order and drops unknown ids', () => {
    expect(attachedRecords(['acc-0', 'nope', 'acc-1'], records)).toEqual([superseded, accepted]);
  });
});

describe('promptLabel', () => {
  it('asks a different question per mode', () => {
    expect(promptLabel('new')).toBe(
      'What is the decision? Or say what to look for in the attached files.',
    );
    expect(promptLabel('revise')).toBe('What should change?');
    expect(promptLabel('supersede')).toBe('What changed, and why?');
    expect(promptLabel('merge')).toBe('Anything the merged decision should emphasize? (optional)');
  });
});

describe('attachRecordsStatus', () => {
  it('starts the picker on the status the mode needs', () => {
    expect(attachRecordsStatus('merge')).toBe('candidate');
    expect(attachRecordsStatus('supersede')).toBe('accepted');
    expect(attachRecordsStatus('new')).toBe('all');
    expect(attachRecordsStatus('revise')).toBe('all');
  });
});

describe('views', () => {
  it('offers the manual form only when the outcome is a new candidate', () => {
    expect(manualFormAvailable('new')).toBe(true);
    expect(manualFormAvailable('supersede')).toBe(true);
    expect(manualFormAvailable('revise')).toBe(false);
    expect(manualFormAvailable('merge')).toBe(false);
  });

  it('starts on the agent when one is available, else on the manual form', () => {
    expect(initialComposerView('new', true)).toBe('agent');
    expect(initialComposerView('new', false)).toBe('manual');
    expect(initialComposerView('supersede', false)).toBe('manual');
    expect(initialComposerView('revise', false)).toBe('agent');
    expect(initialComposerView('merge', false)).toBe('agent');
  });
});

const polarsSource = [
  '---',
  'title: Use Polars',
  'status: accepted',
  'id: acc-1',
  'tags:',
  '  - "#dadr/data"',
  '  - kind/setup',
  '---',
  '',
  '## Supersedes',
  '',
  '[[0193fb1f-a3a0-7000-8000-000000000009|Use pandas]]',
  '',
  'Pandas ran out of memory.',
  '',
  '## Context',
  '',
  'Data frames.',
  '',
  '## Decision',
  '',
  'Use Polars.',
  '',
].join('\n');

describe('successorPrefill', () => {
  it('starts a successor from the exact title, tags, and body, without its own Supersedes', () => {
    expect(successorStartingBody(polarsSource)).toBe('## Context\n\nData frames.\n\n## Decision\n\nUse Polars.\n');
    expect(successorPrefill([{ record: accepted, source: polarsSource }])).toEqual({
      title: 'Use Polars',
      tags: '#dadr/data kind/setup',
      body: '## Context\n\nData frames.\n\n## Decision\n\nUse Polars.\n',
    });
  });

  it('falls back to the summary tags when the file has none', () => {
    const plain = '---\ntitle: Use Polars\nstatus: accepted\nid: acc-1\n---\n\nBody.\n';
    expect(successorPrefill([{ record: accepted, source: plain }]).tags).toBe('#dadr/data #kind/setup');
  });

  it('starts a merge untitled, with every tag and each section from every predecessor', () => {
    const other = '---\ntitle: Use pandas\nstatus: accepted\nid: acc-0\ntags: [kind/setup, topic/memory]\n---\n\n## Context\n\nMemory.\n\n## Decision\n\nUse pandas.\n';
    const merged = successorPrefill([
      { record: accepted, source: polarsSource },
      { record: superseded, source: other },
    ]);
    expect(merged.title).toBe('');
    expect(merged.tags).toBe('#dadr/data kind/setup topic/memory');
    expect(merged.body).toBe(
      '## Context\n\nData frames.\n\nMemory.\n\n## Decision\n\nUse Polars.\n\nUse pandas.\n\n## Alternatives\n\n## Consequences\n',
    );
  });
});

describe('successorChanged', () => {
  const start = { title: 'Use Polars', tags: 'a b', body: '## Decision\n\nUse Polars.\n' };

  it('ignores whitespace that does not change the text', () => {
    expect(successorChanged(start, { ...start, tags: ' a  b ', body: '\n## Decision\n\nUse Polars.\n\n' })).toBe(false);
  });

  it('notices a change to the title, the tags, or the body', () => {
    expect(successorChanged(start, { ...start, title: 'Use DuckDB' })).toBe(true);
    expect(successorChanged(start, { ...start, tags: 'a' })).toBe(true);
    expect(successorChanged(start, { ...start, body: '## Decision\n\nUse Polars lazily.\n' })).toBe(true);
  });
});

describe('ledgerEditHref', () => {
  it('opens the edit tab, showing archived records for records that are not current', () => {
    expect(ledgerEditHref(accepted)).toBe('/ledger?id=acc-1&tab=edit');
    expect(ledgerEditHref(candidate)).toBe('/ledger?archived=1&id=cand-1&tab=edit');
    expect(ledgerEditHref(superseded)).toBe('/ledger?archived=1&id=acc-0&tab=edit');
  });
});

describe('errorMessage', () => {
  it('reads errors, strings, and anything else', () => {
    expect(errorMessage(new Error('boom'))).toBe('boom');
    expect(errorMessage('plain')).toBe('plain');
    expect(errorMessage({ code: 1 })).toBe('Unknown error');
  });
});
