/**
 * Just enough YAML front matter handling for the web UI: split a record
 * source into front matter and body, read the DADR keys, and rewrite `tags`
 * while leaving every other line exactly as written.
 */

export interface SourceParts {
  /** The front matter with both `---` delimiters and the closing line ending; empty when there is none. */
  frontMatter: string;
  /** Everything after the closing delimiter. */
  body: string;
}

export interface FrontMatterBlock {
  key: string;
  /** The key line and its continuation lines, without line endings. */
  lines: string[];
}

export interface FrontMatterFields {
  id?: string;
  title?: string;
  status?: string;
  tags: string[];
  successors: string[];
}

/** Split a record source after the closing `---` line. Joining the parts gives the source back. */
export function splitSource(source: string): SourceParts {
  const lines = source.split('\n');
  if ((lines[0] ?? '').trim() !== '---') return { frontMatter: '', body: source };
  const close = lines.findIndex((line, index) => index > 0 && line.trim() === '---');
  if (close < 0) return { frontMatter: '', body: source };
  const frontMatter = lines.slice(0, close + 1).join('\n');
  const rest = lines.slice(close + 1).join('\n');
  // The closing line's newline belongs to the front matter.
  return close + 1 < lines.length
    ? { frontMatter: `${frontMatter}\n`, body: rest }
    : { frontMatter, body: '' };
}

/** The top-level keys of the front matter, each with its continuation lines. */
export function frontMatterBlocks(source: string): FrontMatterBlock[] {
  return blocksOf(innerLines(source));
}

/** The DADR keys of a record source. Unknown and malformed values are left out. */
export function readFrontMatter(source: string): FrontMatterFields {
  const fields: FrontMatterFields = { tags: [], successors: [] };
  for (const block of frontMatterBlocks(source)) {
    const value = blockValue(block);
    switch (block.key) {
      case 'id':
      case 'title':
      case 'status':
        if (typeof value === 'string') fields[block.key] = value;
        break;
      case 'tags':
      case 'successors':
        fields[block.key] = Array.isArray(value) ? value : typeof value === 'string' && value ? [value] : [];
        break;
    }
  }
  return fields;
}

/**
 * The source with its `tags` replaced by `tags`, written as a block list (or
 * `tags: []`). Every other line, including the body, is kept as written; a
 * source without `tags` gains the key just before the closing delimiter.
 */
export function rewriteTags(source: string, tags: string[]): string {
  const { frontMatter, body } = splitSource(source);
  if (!frontMatter) throw new Error('the record has no front matter');
  const eol = frontMatter.includes('\r\n') ? '\r\n' : '\n';
  const lines = frontMatter.replace(/\r\n/g, '\n').replace(/\n$/, '').split('\n');
  const inner = lines.slice(1, -1);
  const replacement = tags.length ? ['tags:', ...tags.map((tag) => `  - ${yamlScalar(tag)}`)] : ['tags: []'];

  const start = inner.findIndex((line) => keyOf(line) === 'tags');
  let next: string[];
  if (start < 0) {
    next = [...inner, ...replacement];
  } else {
    let end = start + 1;
    while (end < inner.length && isContinuation(inner[end] ?? '')) end += 1;
    next = [...inner.slice(0, start), ...replacement, ...inner.slice(end)];
  }
  const rewritten = [lines[0], ...next, lines.at(-1)].join(eol);
  return `${rewritten}${eol}${body}`;
}

/** A tag as a YAML scalar: plain when that is unambiguous, double-quoted otherwise. */
export function yamlScalar(value: string): string {
  return /^[A-Za-z0-9_][A-Za-z0-9_./-]*$/.test(value) ? value : JSON.stringify(value);
}

function innerLines(source: string): string[] {
  const { frontMatter } = splitSource(source);
  if (!frontMatter) return [];
  const lines = frontMatter.replace(/\r\n/g, '\n').replace(/\n$/, '').split('\n');
  return lines.slice(1, -1);
}

function blocksOf(lines: string[]): FrontMatterBlock[] {
  const blocks: FrontMatterBlock[] = [];
  for (const line of lines) {
    const key = keyOf(line);
    if (key !== null) {
      blocks.push({ key, lines: [line] });
    } else if (blocks.length && isContinuation(line)) {
      blocks.at(-1)?.lines.push(line);
    } else if (blocks.length && !line.trim()) {
      blocks.at(-1)?.lines.push(line);
    }
  }
  return blocks;
}

function keyOf(line: string): string | null {
  const match = line.match(/^([A-Za-z_][A-Za-z0-9_-]*)\s*:(?:\s|$)/);
  return match ? (match[1] ?? null) : null;
}

/** Indented lines and block-sequence items belong to the key above them. */
function isContinuation(line: string): boolean {
  return /^[ \t]/.test(line) || /^-(\s|$)/.test(line);
}

function blockValue(block: FrontMatterBlock): string | string[] | null {
  const first = block.lines[0] ?? '';
  const inline = first.slice(first.indexOf(':') + 1).trim();
  const rest = block.lines.slice(1);
  if (!inline || inline.startsWith('#')) {
    const items = rest
      .map((line) => line.trim())
      .filter((line) => line.startsWith('-'))
      .map((line) => unquote(stripComment(line.slice(1).trim())))
      .filter(Boolean);
    return items.length || rest.some((line) => line.trim().startsWith('-')) ? items : null;
  }
  if (inline.startsWith('[')) {
    const flow = [inline, ...rest.map((line) => line.trim())].join(' ');
    const close = flow.lastIndexOf(']');
    const content = flow.slice(1, close < 0 ? undefined : close);
    return splitFlow(content).map(unquote).filter(Boolean);
  }
  return unquote(stripComment(inline));
}

function splitFlow(content: string): string[] {
  const items: string[] = [];
  let current = '';
  let quote: string | null = null;
  for (const char of content) {
    if (quote) {
      current += char;
      if (char === quote) quote = null;
    } else if (char === '"' || char === "'") {
      quote = char;
      current += char;
    } else if (char === ',') {
      items.push(current.trim());
      current = '';
    } else {
      current += char;
    }
  }
  if (current.trim()) items.push(current.trim());
  return items;
}

function stripComment(value: string): string {
  if (value.startsWith('"') || value.startsWith("'")) return value;
  const hash = value.search(/\s#/);
  return hash < 0 ? value : value.slice(0, hash).trimEnd();
}

function unquote(value: string): string {
  const trimmed = value.trim();
  if (trimmed.length >= 2 && trimmed.startsWith('"') && trimmed.endsWith('"')) {
    try {
      return JSON.parse(trimmed) as string;
    } catch {
      return trimmed.slice(1, -1);
    }
  }
  if (trimmed.length >= 2 && trimmed.startsWith("'") && trimmed.endsWith("'")) {
    return trimmed.slice(1, -1).replaceAll("''", "'");
  }
  return trimmed;
}
