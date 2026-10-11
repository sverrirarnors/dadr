export type DiffLineKind = 'same' | 'add' | 'remove';

export interface DiffLine {
  kind: DiffLineKind;
  text: string;
}

export interface DiffStats {
  added: number;
  removed: number;
}

/**
 * Line-level diff between two Markdown documents, built from the longest
 * common subsequence of lines. Removed lines come before the added lines that
 * replace them, which is the conventional reading order.
 */
export function lineDiff(before: string, after: string): DiffLine[] {
  const left = splitLines(before);
  const right = splitLines(after);
  const rows = left.length;
  const cols = right.length;
  const width = cols + 1;
  // lcs[i][j] = length of the LCS of left[i..] and right[j..], stored flat.
  const lcs = new Uint32Array((rows + 1) * width);
  const at = (i: number, j: number) => lcs[i * width + j] ?? 0;

  for (let i = rows - 1; i >= 0; i -= 1) {
    for (let j = cols - 1; j >= 0; j -= 1) {
      lcs[i * width + j] =
        left[i] === right[j] ? at(i + 1, j + 1) + 1 : Math.max(at(i + 1, j), at(i, j + 1));
    }
  }

  const lines: DiffLine[] = [];
  let i = 0;
  let j = 0;
  while (i < rows && j < cols) {
    const leftLine = left[i] ?? '';
    const rightLine = right[j] ?? '';
    if (leftLine === rightLine) {
      lines.push({ kind: 'same', text: leftLine });
      i += 1;
      j += 1;
    } else if (at(i + 1, j) >= at(i, j + 1)) {
      lines.push({ kind: 'remove', text: leftLine });
      i += 1;
    } else {
      lines.push({ kind: 'add', text: rightLine });
      j += 1;
    }
  }
  for (; i < rows; i += 1) lines.push({ kind: 'remove', text: left[i] ?? '' });
  for (; j < cols; j += 1) lines.push({ kind: 'add', text: right[j] ?? '' });
  return lines;
}

export function diffStats(lines: DiffLine[]): DiffStats {
  return lines.reduce<DiffStats>(
    (stats, line) => {
      if (line.kind === 'add') stats.added += 1;
      else if (line.kind === 'remove') stats.removed += 1;
      return stats;
    },
    { added: 0, removed: 0 },
  );
}

export function hasChanges(lines: DiffLine[]): boolean {
  return lines.some((line) => line.kind !== 'same');
}

function splitLines(text: string): string[] {
  const normalized = text.replace(/\r\n/g, '\n');
  if (!normalized) return [];
  const lines = normalized.split('\n');
  // A trailing newline ends the last line; it is not an extra empty line.
  if (lines.length > 1 && lines.at(-1) === '') lines.pop();
  return lines;
}

/** The kinds Dadrock's diff endpoints use for a line. */
export interface RecordDiffLine {
  kind: 'context' | 'added' | 'removed';
  text: string;
}

/** Lines from `dadrock diff` and `dadrock history --patch`, in the form `LineDiff` draws. */
export function fromRecordDiffLines(lines: RecordDiffLine[]): DiffLine[] {
  return lines.map((line) => ({
    kind: line.kind === 'context' ? 'same' : line.kind === 'added' ? 'add' : 'remove',
    text: line.text,
  }));
}

/** Both texts a full line diff was built from: unchanged and removed lines, unchanged and added lines. */
export function diffSides(lines: DiffLine[]): { before: string; after: string } {
  const side = (keep: DiffLineKind) =>
    lines
      .filter((line) => line.kind === 'same' || line.kind === keep)
      .map((line) => `${line.text}\n`)
      .join('');
  return { before: side('remove'), after: side('add') };
}

/** `+a −b` for the tags a change adds and removes; empty when they are the same. */
export function tagChangeSummary(added: string[], removed: string[]): string {
  return [...added.map((tag) => `+${tag}`), ...removed.map((tag) => `−${tag}`)].join(' ');
}
