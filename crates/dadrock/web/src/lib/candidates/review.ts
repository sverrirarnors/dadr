/** Explicit drafting markers, not a guess at the analyst's intent. */
export function hasMissingRationale(markdown: string): boolean {
  return /rationale not documented/i.test(markdown);
}
