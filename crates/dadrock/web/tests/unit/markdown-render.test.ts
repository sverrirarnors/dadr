import { describe, expect, it } from 'vitest';
import { renderMarkdown, stripFrontMatter } from '../../src/lib/markdown/render';

describe('stripFrontMatter', () => {
  it('removes a leading YAML front matter block', () => {
    expect(stripFrontMatter('---\ntitle: Use Polars\n---\n\n# Use Polars\n\nBody')).toBe(
      '# Use Polars\n\nBody',
    );
  });

  it('leaves ordinary markdown untouched', () => {
    expect(stripFrontMatter('# Heading\n\nBody')).toBe('# Heading\n\nBody');
  });
});

describe('renderMarkdown', () => {
  it('renders headings, lists, inline code, links, and wikilinks', () => {
    const html = renderMarkdown(
      '# Decision\n\nUse `polars` with [[0193fb1fa3a0|the data decision]].\n\n- fast\n- typed\n\n[docs](https://pola.rs)',
    );

    expect(html).toContain('<h1>Decision</h1>');
    expect(html).toContain('<code>polars</code>');
    expect(html).toContain('data-wikilink="0193fb1fa3a0"');
    expect(html).toContain('<li>fast</li>');
    expect(html).toContain('href="https://pola.rs"');
  });

  it('escapes raw HTML before applying inline markdown', () => {
    const html = renderMarkdown('Do not run <script>alert("x")</script>.');

    expect(html).toContain('&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;');
    expect(html).not.toContain('<script>');
  });

  it('renders unsafe link schemes as inert text', () => {
    const html = renderMarkdown(
      '[one](javascript:alert(1)) [two](JaVaScRiPt:alert(2)) [three](%6a%61vascript:alert(3)) [four](data:text/html,test)',
    );

    expect(html).not.toContain('<a href="javascript:');
    expect(html).not.toContain('<a href="JaVaScRiPt:');
    expect(html).not.toContain('<a href="%6a%61vascript:');
    expect(html).not.toContain('<a href="data:');
    expect(html).toContain('[one](javascript:alert(1))');
    expect(html).toContain('[four](data:text/html,test)');
  });
});
