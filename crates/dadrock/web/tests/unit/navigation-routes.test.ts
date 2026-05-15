import { describe, expect, it } from 'vitest';
import { candidateInboxHref, isPrimaryNavItemActive, primaryNavItems } from '../../src/lib/navigation/routes';

function navItem(id: (typeof primaryNavItems)[number]['id']) {
  const item = primaryNavItems.find((candidate) => candidate.id === id);
  if (!item) throw new Error(`missing nav item: ${id}`);
  return item;
}

function url(path: string) {
  return new URL(path, 'http://example.test');
}

describe('primary navigation routes', () => {
  it('uses the candidate records filter for the inbox destination', () => {
    expect(candidateInboxHref).toBe('/records?status=candidate');
    expect(navItem('inbox').href).toBe(candidateInboxHref);
  });

  it('separates the inbox and records active states on the records route', () => {
    expect(isPrimaryNavItemActive(navItem('inbox'), url('/records?status=candidate'))).toBe(true);
    expect(isPrimaryNavItemActive(navItem('records'), url('/records?status=candidate'))).toBe(false);
    expect(isPrimaryNavItemActive(navItem('records'), url('/records'))).toBe(true);
    expect(isPrimaryNavItemActive(navItem('inbox'), url('/records'))).toBe(false);
  });

  it('treats nested primary sections as active', () => {
    expect(isPrimaryNavItemActive(navItem('tasks'), url('/tasks/runs'))).toBe(true);
    expect(isPrimaryNavItemActive(navItem('validation'), url('/validation'))).toBe(true);
  });
});
