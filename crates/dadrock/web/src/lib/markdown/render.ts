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
  const lines = markdown.split(/\r?\n/);
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

  for (const line of lines) {
    if (line.trim().startsWith('```')) {
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

    const heading = line.match(/^(#{1,3})\s+(.*)$/);
    if (heading) {
      flushParagraph();
      closeList();
      const level = heading[1]!.length;
      html.push(`<h${level}>${inlineMarkdown(heading[2]!)}</h${level}>`);
      continue;
    }

    const item = line.match(/^\s*[-*]\s+(.*)$/);
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
