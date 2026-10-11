export function stripFrontMatter(markdown: string): string {
  const normalized = markdown.replace(/\r\n/g, '\n');
  const lines = normalized.split('\n');
  if (lines[0]?.trim() !== '---') return markdown;

  const end = lines.slice(1).findIndex((line) => line.trim() === '---');
  if (end < 0) return markdown;

  return lines
    .slice(end + 2)
    .join('\n')
    .replace(/^\n/, '');
}

export function renderMarkdown(markdown: string): string {
  return renderBlocks(markdown.split(/\r?\n/), 0);
}

// A blockquote line: up to three spaces of indentation, `>`, and one optional space.
const QUOTE_MARKER = /^ {0,3}> ?/;
const NESTED_QUOTE_MARKERS = /^(?: {0,3}> ?)*/;
const HEADING = /^(#{1,3})\s+(.*)$/;
const LIST_ITEM = /^\s*[-*]\s+(.*)$/;
// Deeper `>` lines render as text, so hostile input cannot exhaust the stack.
const MAX_QUOTE_DEPTH = 16;

function renderBlocks(lines: string[], depth: number): string {
  const html: string[] = [];
  let paragraph: string[] = [];
  let listOpen = false;
  let codeOpen = false;
  let codeLines: string[] = [];

  const flushParagraph = () => {
    if (!paragraph.length) return;
    html.push(`<p>${inlineMarkdown(paragraph.join(' '))}</p>`);
    paragraph = [];
  };

  const closeList = () => {
    if (!listOpen) return;
    html.push('</ul>');
    listOpen = false;
  };

  let index = 0;
  while (index < lines.length) {
    const line = lines[index]!;
    index += 1;

    if (isFence(line)) {
      if (codeOpen) {
        html.push(`<pre><code>${escapeHtml(codeLines.join('\n'))}</code></pre>`);
        codeLines = [];
        codeOpen = false;
      } else {
        flushParagraph();
        closeList();
        codeOpen = true;
      }
      continue;
    }

    if (codeOpen) {
      codeLines.push(line);
      continue;
    }

    if (!line.trim()) {
      flushParagraph();
      closeList();
      continue;
    }

    if (depth < MAX_QUOTE_DEPTH && QUOTE_MARKER.test(line)) {
      flushParagraph();
      closeList();
      const quote = collectBlockquote(lines, index - 1);
      html.push(`<blockquote>${renderBlocks(quote.lines, depth + 1)}</blockquote>`);
      index = quote.next;
      continue;
    }

    const heading = line.match(HEADING);
    if (heading) {
      flushParagraph();
      closeList();
      const level = heading[1]!.length;
      html.push(`<h${level}>${inlineMarkdown(heading[2]!)}</h${level}>`);
      continue;
    }

    const item = line.match(LIST_ITEM);
    if (item) {
      flushParagraph();
      if (!listOpen) {
        html.push('<ul>');
        listOpen = true;
      }
      html.push(`<li>${inlineMarkdown(item[1]!)}</li>`);
      continue;
    }

    paragraph.push(line.trim());
  }

  flushParagraph();
  closeList();
  if (codeOpen) {
    html.push(`<pre><code>${escapeHtml(codeLines.join('\n'))}</code></pre>`);
  }

  return html.join('');
}

/**
 * Collects the blockquote that starts at `start`: every following `>` line with its marker
 * removed, plus lazy continuation lines (a line without `>` that continues an open paragraph).
 * A blank line, or any line that would start another block, ends the quote.
 */
function collectBlockquote(lines: string[], start: number): { lines: string[]; next: number } {
  const quoted: string[] = [];
  let fenceOpen = false;
  let index = start;

  while (index < lines.length) {
    const line = lines[index]!;
    const marker = line.match(QUOTE_MARKER);
    if (marker) {
      const content = line.slice(marker[0].length);
      if (isFence(content)) fenceOpen = !fenceOpen;
      quoted.push(content);
    } else if (!fenceOpen && continuesParagraph(quoted.at(-1)) && !startsBlock(line)) {
      quoted.push(line);
    } else {
      break;
    }
    index += 1;
  }

  return { lines: quoted, next: index };
}

/** Whether a quoted line leaves a paragraph open, looking through nested `>` markers. */
function continuesParagraph(line: string | undefined): boolean {
  if (line === undefined) return false;
  const content = line.replace(NESTED_QUOTE_MARKERS, '');
  return Boolean(content.trim()) && !HEADING.test(content) && !isFence(content);
}

function startsBlock(line: string): boolean {
  return !line.trim() || isFence(line) || HEADING.test(line) || LIST_ITEM.test(line);
}

function isFence(line: string): boolean {
  return line.trim().startsWith('```');
}

// Only short-id and full-id targets are DADR record links; other wikilinks stay as text.
const RECORD_LINK =
  /\[\[\s*([0-9a-fA-F]{12}|[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})\s*(?:\|([^\]]+))?\]\]/g;

function inlineMarkdown(text: string): string {
  let escaped = escapeHtml(text);
  escaped = escaped.replace(RECORD_LINK, (_, target: string, label: string) => {
    return `<button class="wikilink" type="button" data-wikilink="${escapeAttr(target)}">${escapeHtml(
      label || target,
    )}</button>`;
  });
  escaped = escaped.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_, label: string, url: string) => {
    if (!isSafeLinkUrl(url)) {
      return `[${label}](${url})`;
    }
    return `<a href="${escapeAttr(url)}" target="_blank" rel="noreferrer">${escapeHtml(label)}</a>`;
  });
  escaped = escaped.replace(/`([^`]+)`/g, '<code>$1</code>');
  return escaped;
}

function escapeHtml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function escapeAttr(value: string): string {
  return escapeHtml(value).replaceAll('`', '&#96;');
}

function isSafeLinkUrl(url: string): boolean {
  const trimmed = url.trim();
  if (!trimmed) return false;
  if (trimmed.startsWith('#')) return true;
  if (trimmed.startsWith('/') && !trimmed.startsWith('//')) return true;
  if (trimmed.startsWith('./') || trimmed.startsWith('../')) return true;

  const normalized = decodedSchemeCandidate(trimmed).toLowerCase();
  const scheme = normalized.match(/^([a-z][a-z0-9+.-]*):/);
  if (scheme) {
    return ['http', 'https', 'mailto'].includes(scheme[1]!);
  }

  return !trimmed.startsWith('//');
}

function decodedSchemeCandidate(url: string): string {
  // eslint-disable-next-line no-control-regex -- Strip leading controls that can conceal an unsafe URL scheme.
  let candidate = url.replace(/^[\u0000-\u0020\u007f]+/, '');
  for (let i = 0; i < 3; i += 1) {
    try {
      const decoded = decodeURIComponent(candidate);
      if (decoded === candidate) break;
      candidate = decoded;
    } catch {
      break;
    }
  }
  // eslint-disable-next-line no-control-regex -- Strip decoded controls before checking URL scheme safety.
  return candidate.replace(/[\u0000-\u0020\u007f]+/g, '');
}
