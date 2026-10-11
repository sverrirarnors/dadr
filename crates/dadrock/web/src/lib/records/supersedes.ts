/**
 * The `## Supersedes` section of a record body, read the way Dadrock reads it
 * (`records::supersedes_section` in the dadrock crate).
 *
 * The section starts with one or more lines that are each exactly one record
 * link by full id, `[[uuid]]` or `[[uuid|label]]`. The first other non-empty
 * line starts the reason, which runs to the end of the section. Headings
 * inside fenced or indented code neither open nor close a section, and code
 * inside the section is never a link line.
 */

export const SUPERSEDES_HEADING = '## Supersedes';

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

type LineKind = 'code' | 'heading' | 'supersedes-heading' | 'text';

interface BodyLine {
  /** The line with its line ending, as it appears in the body. */
  raw: string;
  /** The line without its line ending. */
  text: string;
  kind: LineKind;
}

export interface SupersedesLink {
  id: string;
  label: string | null;
}

export interface SupersedesSection {
  links: SupersedesLink[];
  /** The Markdown after the links, or null when there is none. */
  reason: string | null;
}

export interface SupersedesTarget {
  id: string;
  title: string;
}

/** The parsed `## Supersedes` section, or null when the body has none or it is invalid. */
export function supersedesSection(body: string): SupersedesSection | null {
  const lines = classifyBodyLines(body);
  const sections = sectionRanges(lines);
  if (sections.length !== 1) return null;
  const [start, end] = sections[0] as [number, number];

  const links: SupersedesLink[] = [];
  let reasonStart = end;
  for (let index = start + 1; index < end; index += 1) {
    const line = lines[index] as BodyLine;
    const text = line.text.trim();
    if (!text) continue;
    const link = line.kind === 'text' ? linkTarget(text) : null;
    if (!link) {
      reasonStart = index;
      break;
    }
    if (!link.valid) return null;
    if (links.some((existing) => sameId(existing.id, link.id))) return null;
    links.push({ id: link.id, label: link.label });
  }
  if (!links.length) return null;

  const reasonLines = lines.slice(reasonStart, end).map((line) => line.text);
  while (reasonLines.length && !(reasonLines.at(-1) ?? '').trim()) reasonLines.pop();
  return { links, reason: reasonLines.length ? reasonLines.join('\n') : null };
}

/** The ids a body's `## Supersedes` section names, in order; empty when there is none. */
export function supersededIds(body: string): string[] {
  return supersedesSection(body)?.links.map((link) => link.id) ?? [];
}

/** True when the body has a `## Supersedes` heading, valid or not. */
export function hasSupersedesHeading(body: string): boolean {
  return classifyBodyLines(body).some((line) => line.kind === 'supersedes-heading');
}

/**
 * The body with every `## Supersedes` section removed. The rest of the body is
 * unchanged, except that blank lines left at the end by removing a final
 * section are dropped.
 */
export function bodyWithoutSupersedes(body: string): string {
  const lines = classifyBodyLines(body);
  const sections = sectionRanges(lines);
  if (!sections.length) return body;
  let output = '';
  let next = 0;
  for (const [start, end] of sections) {
    output += lines.slice(next, start).map((line) => line.raw).join('');
    next = end;
  }
  output += lines.slice(next).map((line) => line.raw).join('');
  if (next === lines.length) {
    output = output.trimEnd();
    if (output) output += '\n';
  }
  return output;
}

/**
 * The body as versions are compared: without `## Supersedes`, with `\n` line
 * endings, and without leading or trailing blank lines.
 */
export function comparableBody(body: string): string {
  const normalized = body.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  const lines = bodyWithoutSupersedes(normalized).split('\n');
  if (lines.at(-1) === '') lines.pop();
  while (lines.length && !(lines[0] ?? '').trim()) lines.shift();
  while (lines.length && !(lines.at(-1) ?? '').trim()) lines.pop();
  return lines.length ? `${lines.join('\n')}\n` : '';
}

/**
 * Point the `## Supersedes` link to `from` at `to` instead: the link line is
 * replaced by one link per target, labeled with its title, skipping targets
 * the section already names. Everything else is kept as written. Returns null
 * when the section does not link `from`.
 */
export function retargetSupersedes(body: string, from: string, to: SupersedesTarget[]): string | null {
  const lines = classifyBodyLines(body);
  const sections = sectionRanges(lines);
  if (sections.length !== 1) return null;
  const [start, end] = sections[0] as [number, number];

  const named: string[] = [];
  let found = -1;
  for (let index = start + 1; index < end; index += 1) {
    const line = lines[index] as BodyLine;
    const text = line.text.trim();
    if (!text) continue;
    const link = line.kind === 'text' ? linkTarget(text) : null;
    if (!link) break;
    named.push(link.id);
    if (found < 0 && sameId(link.id, from)) found = index;
  }
  if (found < 0) return null;

  const original = lines[found] as BodyLine;
  const ending = original.raw.slice(original.text.length) || '\n';
  const indent = original.text.match(/^\s*/)?.[0] ?? '';
  const others = named.filter((id) => !sameId(id, from));
  const targets: SupersedesTarget[] = [];
  for (const target of to) {
    if (others.some((id) => sameId(id, target.id))) continue;
    if (targets.some((kept) => sameId(kept.id, target.id))) continue;
    targets.push(target);
  }
  const replacement = targets.map((target) => `${indent}${supersedesLink(target)}`);
  const last = found === lines.length - 1 && !original.raw.endsWith('\n');
  const rewritten = replacement
    .map((line, index) => (last && index === replacement.length - 1 ? line : `${line}${ending}`))
    .join('');
  return [
    ...lines.slice(0, found).map((line) => line.raw),
    rewritten,
    ...lines.slice(found + 1).map((line) => line.raw),
  ].join('');
}

/** One `## Supersedes` link line, labeled with the record's title. */
export function supersedesLink(target: SupersedesTarget): string {
  // A label is one line and cannot contain the link delimiters.
  const label = target.title.replace(/[\r\n]+/g, ' ').replaceAll('[[', '[').replaceAll(']]', ']').trim();
  return label ? `[[${target.id}|${label}]]` : `[[${target.id}]]`;
}

/** Level-1 and level-2 ATX headings of a body outside code, with their line index. */
export function bodyHeadings(body: string): Array<{ index: number; level: number; title: string }> {
  return classifyBodyLines(body).flatMap((line, index) => {
    if (line.kind !== 'heading' && line.kind !== 'supersedes-heading') return [];
    const trimmed = line.text.trimEnd();
    const level = trimmed.match(/^#+/)?.[0].length ?? 0;
    return [{ index, level, title: trimmed.slice(level).trim() }];
  });
}

/** The body split into lines without their line endings, as `bodyHeadings` counts them. */
export function bodyLineTexts(body: string): string[] {
  return classifyBodyLines(body).map((line) => line.text);
}

function sameId(left: string, right: string): boolean {
  return left.toLowerCase() === right.toLowerCase();
}

function linkTarget(text: string): { id: string; label: string | null; valid: boolean } | null {
  if (!text.startsWith('[[') || !text.endsWith(']]')) return null;
  const inner = text.slice(2, -2);
  if (inner.includes('[[') || inner.includes(']]')) return null;
  const bar = inner.indexOf('|');
  const id = (bar < 0 ? inner : inner.slice(0, bar)).trim();
  const label = bar < 0 ? null : inner.slice(bar + 1).trim() || null;
  return { id, label, valid: UUID.test(id) };
}

function classifyBodyLines(body: string): BodyLine[] {
  const lines: BodyLine[] = [];
  let fence: { marker: string; count: number } | null = null;
  let start = 0;
  const push = (raw: string) => {
    let text = raw.endsWith('\n') ? raw.slice(0, -1) : raw;
    if (text.endsWith('\r')) text = text.slice(0, -1);
    const classified = classifyBodyLine(text, fence);
    fence = classified.fence;
    lines.push({ raw, text, kind: classified.kind });
  };
  for (let index = 0; index < body.length; index += 1) {
    if (body[index] === '\n') {
      push(body.slice(start, index + 1));
      start = index + 1;
    }
  }
  if (start < body.length) push(body.slice(start));
  return lines;
}

function classifyBodyLine(
  text: string,
  fence: { marker: string; count: number } | null,
): { kind: LineKind; fence: { marker: string; count: number } | null } {
  // Indented code cannot open or close a fence or a section.
  if (text.startsWith('    ') || text.startsWith('\t')) return { kind: 'code', fence };
  const line = text.trimEnd();
  const fenceLine = line.trimStart();
  const first = fenceLine[0];
  const marker = first === '`' || first === '~' ? first : null;
  let count = 0;
  if (marker) while (fenceLine[count] === marker) count += 1;
  if (fence) {
    const closes = marker === fence.marker && count >= fence.count && !fenceLine.slice(count).trim();
    return { kind: 'code', fence: closes ? null : fence };
  }
  if (marker && count >= 3) return { kind: 'code', fence: { marker, count } };
  let level = 0;
  while (line[level] === '#') level += 1;
  if (level >= 1 && level <= 2 && (line.length === level || line[level] === ' ' || line[level] === '\t')) {
    return { kind: line === SUPERSEDES_HEADING ? 'supersedes-heading' : 'heading', fence: null };
  }
  return { kind: 'text', fence: null };
}

/** Line ranges of each `## Supersedes` section: the heading through the line before the next heading. */
function sectionRanges(lines: BodyLine[]): Array<[number, number]> {
  const sections: Array<[number, number]> = [];
  let open: number | null = null;
  lines.forEach((line, index) => {
    if (line.kind !== 'heading' && line.kind !== 'supersedes-heading') return;
    if (open !== null) sections.push([open, index]);
    open = line.kind === 'supersedes-heading' ? index : null;
  });
  if (open !== null) sections.push([open, lines.length]);
  return sections;
}
