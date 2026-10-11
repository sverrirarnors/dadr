import { describe, expect, it } from 'vitest';
import type { RecordSummary } from '../../src/lib/api';
import {
  attachRecordsStatus,
  composerTitle,
  errorMessage,
  initialComposerView,
  ledgerEditHref,
  manualFormAvailable,
  manualPrefill,
  pinnedRecord,
  promptLabel,
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
    expect(composerTitle('new', undefined)).toBe('Draft a decision');
    expect(composerTitle('merge', undefined)).toBe('Merge candidates');
    expect(composerTitle('revise', candidate)).toBe('Revise “First draft”');
    expect(composerTitle('supersede', accepted)).toBe('Replace “Use Polars”');
  });

  it('falls back when the record is unknown', () => {
    expect(composerTitle('revise', undefined)).toBe('Revise a record');
    expect(composerTitle('supersede', undefined)).toBe('Replace a decision');
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

describe('manualPrefill', () => {
  it('prefills a replacement from the original decision', () => {
    expect(manualPrefill('supersede', accepted)).toEqual({
      title: 'Update: Use Polars',
      tags: '#dadr/data #kind/setup',
    });
  });

  it('starts blank otherwise', () => {
    expect(manualPrefill('new', undefined)).toEqual({ title: '', tags: '' });
    expect(manualPrefill('new', accepted)).toEqual({ title: '', tags: '' });
    expect(manualPrefill('supersede', undefined)).toEqual({ title: '', tags: '' });
  });
});

describe('ledgerEditHref', () => {
  it('opens the edit tab, showing history for records that are not current', () => {
    expect(ledgerEditHref(accepted)).toBe('/ledger?id=acc-1&tab=edit');
    expect(ledgerEditHref(candidate)).toBe('/ledger?history=1&id=cand-1&tab=edit');
    expect(ledgerEditHref(superseded)).toBe('/ledger?history=1&id=acc-0&tab=edit');
  });
});

describe('errorMessage', () => {
  it('reads errors, strings, and anything else', () => {
    expect(errorMessage(new Error('boom'))).toBe('boom');
    expect(errorMessage('plain')).toBe('plain');
    expect(errorMessage({ code: 1 })).toBe('Unknown error');
  });
});
