import { describe, expect, it } from 'vitest';
import { candidateDestination, parseListInput } from '../../src/lib/candidates/form';
import type { MutationResponse } from '../../src/lib/api';

describe('parseListInput', () => {
  it('splits comma and whitespace separated values while preserving hashes', () => {
    expect(parseListInput(' #dadr/data/processing, #kind/model\n#team ')).toEqual([
      '#dadr/data/processing',
      '#kind/model',
      '#team',
    ]);
  });

  it('drops empty items', () => {
    expect(parseListInput(' ,  ,')).toEqual([]);
  });
});

describe('candidateDestination', () => {
  it('routes to the selected candidate when mutation focus includes an id', () => {
    const response = {
      focus: {
        selected_id: '0193fb1f-a3a0-7000-8000-000000000003',
        status_filter: 'candidate',
      },
    } as MutationResponse;

    expect(candidateDestination(response)).toEqual({
      route: '/records/[id]',
      params: { id: '0193fb1f-a3a0-7000-8000-000000000003' },
    });
  });

  it('falls back to the candidate record list when no selected id exists', () => {
    expect(candidateDestination({} as MutationResponse)).toEqual({
      route: '/records',
      query: '?status=candidate',
    });
  });
});
