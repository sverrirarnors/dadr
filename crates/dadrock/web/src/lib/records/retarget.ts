import { splitSource } from './front-matter';
import { retargetSupersedes, type SupersedesTarget } from './supersedes';

/**
 * A candidate's file content with the `## Supersedes` link to a superseded
 * predecessor pointed at the decisions now in force. The front matter and
 * the rest of the body are kept as written.
 */
export function retargetSource(source: string, from: string, to: SupersedesTarget[]): string {
  if (!to.length) throw new Error('Choose the decision to point at.');
  const { frontMatter, body } = splitSource(source);
  const rewritten = retargetSupersedes(body, from, to);
  if (rewritten === null) throw new Error('`## Supersedes` in this candidate does not link that record any more.');
  return `${frontMatter}${rewritten}`;
}
