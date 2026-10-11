import { describe, expect, it } from 'vitest';
import { candidateDestination, parseListInput } from '../../src/lib/candidates/form';
import type { MutationResponse } from '../../src/lib/api';

describe('parseListInput', () => {
  it('splits comma and whitespace separated values while preserving hashes', () => {
    expect(parseListInput(' #dadr/data/processing, #kind/modeling\n#team ')).toEqual([
      '#dadr/data/processing',
      '#kind/modeling',
      '#team',
    ]);
  });

  it('drops empty items', () => {
    expect(parseListInput(' ,  ,')).toEqual([]);
  });
});

describe('candidateDestination', () => {
  it('selects the new candidate in the inbox when the response names it', () => {
    const response = {
      focus: { selected_id: '0193fb1f-a3a0-7000-8000-000000000003', status_filter: 'candidate' },
    } as MutationResponse;

    expect(candidateDestination(response)).toBe(
      '/inbox?item=candidate%3A0193fb1f-a3a0-7000-8000-000000000003',
    );
  });

  it('falls back to the inbox when no selected id exists', () => {
    expect(candidateDestination({} as MutationResponse)).toBe('/inbox');
  });
});
