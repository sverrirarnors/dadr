import { describe, expect, it } from 'vitest';
import { hasMissingRationale } from '../../src/lib/candidates/review';
import { taskActionLabel } from '../../src/lib/tasks/presentation';
import { primaryNavItems, secondaryNavItems } from '../../src/lib/navigation/routes';

describe('decision review vocabulary', () => {
  it('makes ordinary destinations primary while keeping all advanced tools', () => {
    expect(primaryNavItems.map(item => item.label)).toEqual(['To review', 'Decisions']);
    expect(secondaryNavItems.map(item => item.id)).toEqual(['tags', 'tasks', 'validation', 'settings']);
  });
  it('surfaces explicit uncertainty without guessing that every candidate lacks a reason', () => {
    expect(hasMissingRationale('## Decision\nRationale not documented.')).toBe(true);
    expect(hasMissingRationale('## Decision\nUse robust standard errors because of heteroskedasticity.')).toBe(false);
  });
  it('uses familiar actions and preserves custom task names', () => {
    expect(taskActionLabel({ manifest: { id: 'code-to-record', title: 'Code to record' } })).toBe('Find decisions in my code');
    expect(taskActionLabel({ manifest: { id: 'local-task', title: 'My workflow' } })).toBe('My workflow');
  });
});
