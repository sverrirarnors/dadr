import { describe, expect, it } from 'vitest';
import { hasMissingRationale } from '../../src/lib/candidates/review';

describe('decision review', () => {
  it('surfaces explicit uncertainty without guessing that every candidate lacks a reason', () => {
    expect(hasMissingRationale('## Decision\nRationale not documented.')).toBe(true);
    expect(hasMissingRationale('## Decision\nUse robust standard errors because of heteroskedasticity.')).toBe(false);
  });
});
