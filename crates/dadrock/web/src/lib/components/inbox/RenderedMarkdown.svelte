<script lang="ts">
  import { renderMarkdown } from '$lib/markdown/render';
  import { cn } from '$lib/utils';

  let { markdown, class: className = '' }: { markdown: string; class?: string } = $props();
  const html = $derived(renderMarkdown(markdown));
</script>

<div data-inbox-markdown class={cn('inbox-markdown text-sm leading-6 text-fg', className)}>
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html html}
</div>

<style>
  :global(.inbox-markdown h1) {
    margin-bottom: 0.75rem;
    font-size: 1.25rem;
    font-weight: 650;
  }

  :global(.inbox-markdown h2) {
    margin-top: 1.25rem;
    margin-bottom: 0.375rem;
    font-size: 0.95rem;
    font-weight: 650;
  }

  :global(.inbox-markdown h2:first-child) {
    margin-top: 0;
  }

  :global(.inbox-markdown h3) {
    margin-top: 1rem;
    margin-bottom: 0.25rem;
    font-weight: 600;
  }

  :global(.inbox-markdown p + p) {
    margin-top: 0.5rem;
  }

  :global(.inbox-markdown ul) {
    margin: 0.375rem 0;
    padding-left: 1.25rem;
    list-style: disc;
  }

  :global(.inbox-markdown blockquote) {
    margin: 0.5rem 0;
    border-left: 3px solid var(--color-border);
    padding-left: 0.75rem;
    color: var(--color-fg-muted);
  }

  :global(.inbox-markdown code) {
    font-family: var(--font-mono);
    font-size: 0.85em;
  }

  :global(.inbox-markdown pre) {
    margin: 0.5rem 0;
    overflow: auto;
    border-radius: var(--radius);
    background: var(--color-surface);
    padding: 0.5rem 0.75rem;
  }

  :global(.inbox-markdown a) {
    color: var(--color-accent);
    text-decoration: underline;
  }
</style>
