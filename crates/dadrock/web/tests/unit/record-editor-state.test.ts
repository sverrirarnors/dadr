import { describe, expect, it } from 'vitest';
import {
  syncRecordEditorSource,
  type RecordEditorState,
} from '../../src/lib/records/editor-state';

const dirtyState: RecordEditorState = {
  sourceId: 'record-a',
  editor: 'dirty local markdown',
  cleanContent: 'original markdown',
  saveMessage: 'saved record-a',
};

describe('syncRecordEditorSource', () => {
  it('keeps dirty editor content when the loaded source is for the same record', () => {
    const next = syncRecordEditorSource(dirtyState, {
      id: 'record-a',
      content: 'server markdown changed',
    });

    expect(next).toBe(dirtyState);
  });

  it('replaces dirty editor content when a different record source loads', () => {
    const next = syncRecordEditorSource(dirtyState, {
      id: 'record-b',
      content: 'record b markdown',
    });

    expect(next).toEqual({
      sourceId: 'record-b',
      editor: 'record b markdown',
      cleanContent: 'record b markdown',
      saveMessage: '',
    });
  });

  it('refreshes clean editor content when the same record is not dirty', () => {
    const cleanState: RecordEditorState = {
      sourceId: 'record-a',
      editor: 'old markdown',
      cleanContent: 'old markdown',
      saveMessage: 'saved record-a',
    };

    const next = syncRecordEditorSource(cleanState, {
      id: 'record-a',
      content: 'fresh markdown',
    });

    expect(next).toEqual({
      sourceId: 'record-a',
      editor: 'fresh markdown',
      cleanContent: 'fresh markdown',
      saveMessage: 'saved record-a',
    });
  });
});
