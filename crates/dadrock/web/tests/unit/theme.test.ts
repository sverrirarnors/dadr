import { beforeEach, describe, expect, it } from 'vitest';
import { theme } from '../../src/lib/stores/theme.svelte';

describe('theme store', () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
    document.documentElement.removeAttribute('data-prefers-dark');
    theme.set('system');
  });

  it('defaults to system when nothing is stored', () => {
    expect(theme.current).toBe('system');
  });

  it('persists explicit choice and applies data-theme', () => {
    theme.set('dark');
    expect(localStorage.getItem('dadr.theme')).toBe('dark');
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  it('cycles system -> light -> dark -> system', () => {
    theme.set('system');
    theme.cycle();
    expect(theme.current).toBe('light');
    theme.cycle();
    expect(theme.current).toBe('dark');
    theme.cycle();
    expect(theme.current).toBe('system');
  });
});
