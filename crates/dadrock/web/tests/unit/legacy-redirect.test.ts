import { describe, expect, it } from 'vitest';
import { legacyRedirectTarget } from '../../src/lib/navigation/legacy';

function target(path: string) {
  return legacyRedirectTarget(new URL(path, 'http://example.test'));
}

describe('legacyRedirectTarget', () => {
  it('sends the root and the old candidate queue to the inbox', () => {
    expect(target('/')).toBe('/inbox');
    expect(target('/records?status=candidate')).toBe('/inbox');
    expect(target('/records?status=candidate&id=abc')).toBe('/inbox?item=candidate%3Aabc');
    expect(target('/candidates/abc')).toBe('/inbox?item=candidate%3Aabc');
  });

  it('sends record browsing to the ledger', () => {
    expect(target('/records')).toBe('/ledger');
    expect(target('/records?id=abc&tab=edit')).toBe('/ledger?id=abc&tab=edit');
    expect(target('/records?status=superseded&id=abc')).toBe('/ledger?history=1&id=abc');
    expect(target('/records?scope=tag&q=%23dadr%2Fdata')).toBe('/ledger?tag=%23dadr%2Fdata');
    expect(target('/records/abc')).toBe('/ledger/abc');
    expect(target('/tags')).toBe('/ledger');
  });

  it('opens the composer for drafting routes', () => {
    expect(target('/candidates/new')).toBe('/inbox?draft=new');
    expect(target('/candidates/new?supersedes=abc')).toBe('/ledger?id=abc&draft=supersede&record=abc');
    expect(target('/tasks')).toBe('/inbox?draft=new');
    expect(target('/tasks?task=edit-record&record=abc')).toBe('/ledger?id=abc&draft=revise&record=abc');
    expect(target('/tasks?task=supersede-record&record=abc')).toBe(
      '/ledger?id=abc&draft=supersede&record=abc',
    );
    expect(target('/tasks?task=code-to-record&path=src')).toBe('/inbox?draft=code&path=src');
    expect(target('/tasks?tab=runs')).toBe('/inbox');
  });

  it('folds the read-only destinations into the inbox', () => {
    for (const path of ['/validation', '/validation?tab=links', '/audit', '/links', '/settings']) {
      expect(target(path)).toBe('/inbox');
    }
  });

  it('leaves current routes alone', () => {
    expect(target('/inbox')).toBeNull();
    expect(target('/ledger?id=abc')).toBeNull();
    expect(target('/ledger/abc')).toBeNull();
  });
});
