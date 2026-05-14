export interface RecordEditorState {
  sourceId: string;
  editor: string;
  cleanContent: string;
  saveMessage: string;
}

export interface LoadedRecordSource {
  id: string;
  content: string;
}

export function initialRecordEditorState(): RecordEditorState {
  return {
    sourceId: '',
    editor: '',
    cleanContent: '',
    saveMessage: '',
  };
}

export function isRecordEditorDirty(state: RecordEditorState): boolean {
  return state.editor !== state.cleanContent;
}

export function syncRecordEditorSource(
  state: RecordEditorState,
  source: LoadedRecordSource,
): RecordEditorState {
  const isNewSource = source.id !== state.sourceId;
  if (!isNewSource && isRecordEditorDirty(state)) {
    return state;
  }

  if (
    state.sourceId === source.id &&
    state.editor === source.content &&
    state.cleanContent === source.content
  ) {
    return state;
  }

  return {
    sourceId: source.id,
    editor: source.content,
    cleanContent: source.content,
    saveMessage: isNewSource ? '' : state.saveMessage,
  };
}
