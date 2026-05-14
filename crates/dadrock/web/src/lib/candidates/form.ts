import type { MutationResponse } from '$lib/api';

export interface CandidateDestination {
  route: '/records' | '/records/[id]';
  params?: { id: string };
  query?: string;
}

export function parseListInput(input: string): string[] {
  return input
    .split(/[\s,]+/)
    .map((item) => item.trim())
    .filter(Boolean);
}

export function candidateDestination(response: MutationResponse): CandidateDestination {
  if (response.focus?.selected_id) {
    return {
      route: '/records/[id]',
      params: { id: response.focus.selected_id },
    };
  }
  return {
    route: '/records',
    query: '?status=candidate',
  };
}
