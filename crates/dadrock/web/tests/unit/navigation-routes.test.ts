import { describe, expect, it } from 'vitest';
import {
  homeHref,
  inboxUrl,
  isPrimaryNavItemActive,
  ledgerRecordUrl,
  ledgerUrl,
  primaryNavItems,
} from '../../src/lib/navigation/routes';

function navItem(id: (typeof primaryNavItems)[number]['id']) {
  const item = primaryNavItems.find((candidate) => candidate.id === id);
  if (!item) throw new Error(`missing nav item: ${id}`);
  return item;
}

function url(path: string) {
  return new URL(path, 'http://example.test');
}

describe('primary navigation', () => {
  it('has exactly two destinations: Inbox and Ledger', () => {
    expect(primaryNavItems.map((item) => item.label)).toEqual(['Inbox', 'Ledger']);
    expect(homeHref).toBe('/inbox');
  });

  it('marks nested routes as active', () => {
    expect(isPrimaryNavItemActive(navItem('inbox'), url('/inbox?item=candidate:x'))).toBe(true);
    expect(isPrimaryNavItemActive(navItem('ledger'), url('/ledger/0193'))).toBe(true);
    expect(isPrimaryNavItemActive(navItem('ledger'), url('/inbox'))).toBe(false);
  });
});

describe('url builders', () => {
  it('keeps ledger urls canonical', () => {
    expect(ledgerUrl()).toBe('/ledger');
    expect(ledgerUrl({ q: '', tag: null, history: false })).toBe('/ledger');
    expect(ledgerUrl({ q: 'polars', tag: '#dadr/data', history: true, id: 'abc', tab: 'edit' })).toBe(
      '/ledger?q=polars&tag=%23dadr%2Fdata&history=1&id=abc&tab=edit',
    );
    expect(ledgerRecordUrl('a b')).toBe('/ledger/a%20b');
  });

  it('builds inbox urls with an optional selected item', () => {
    expect(inboxUrl()).toBe('/inbox');
    expect(inboxUrl('candidate:abc')).toBe('/inbox?item=candidate%3Aabc');
  });
});
