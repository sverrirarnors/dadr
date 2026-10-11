import { bodyHeadings, bodyLineTexts } from './supersedes';

/** The sections every accepted record has, in the order a record lists them. */
export const REQUIRED_SECTIONS = ['Context', 'Decision', 'Alternatives', 'Consequences'] as const;

/** The order a merge is compared in: what was decided first. */
export const COMPARE_SECTIONS = ['Decision', 'Context', 'Alternatives', 'Consequences'] as const;

export type RequiredSection = (typeof REQUIRED_SECTIONS)[number];

export interface SectionSource {
  id: string;
  title: string;
  body: string;
}

export interface SectionText {
  id: string;
  title: string;
  /** The section's text, or null when the record has no such section. */
  text: string | null;
}

export interface SectionComparison {
  section: RequiredSection;
  predecessors: SectionText[];
  /** The successor's text, or null when it has no such section. */
  successor: string | null;
}

/**
 * The level-2 sections of a body by heading, in order, each with its text
 * trimmed of surrounding blank lines. Headings in code do not count. A
 * repeated heading keeps its first section.
 */
export function bodySections(body: string): Map<string, string> {
  const lines = bodyLineTexts(body);
  const headings = bodyHeadings(body);
  const sections = new Map<string, string>();
  headings.forEach((heading, index) => {
    if (heading.level !== 2 || sections.has(heading.title)) return;
    const end = headings[index + 1]?.index ?? lines.length;
    sections.set(heading.title, trimBlankLines(lines.slice(heading.index + 1, end)).join('\n'));
  });
  return sections;
}

/**
 * For each required section, Decision first, each predecessor's text followed
 * by the successor's: the default view of a merge.
 */
export function compareSections(predecessors: SectionSource[], successorBody: string): SectionComparison[] {
  const successor = bodySections(successorBody);
  const sources = predecessors.map((predecessor) => ({ ...predecessor, sections: bodySections(predecessor.body) }));
  return COMPARE_SECTIONS.map((section) => ({
    section,
    predecessors: sources.map((source) => ({
      id: source.id,
      title: source.title,
      text: source.sections.get(section) ?? null,
    })),
    successor: successor.get(section) ?? null,
  }));
}

/**
 * A starting body for a successor that merges several records: each section,
 * required ones first in the usual order, with every predecessor's text for
 * it one after the other.
 */
export function mergedSuccessorBody(bodies: string[]): string {
  const parsed = bodies.map(bodySections);
  // Text before the first heading has no section; keep it at the top.
  const preambles = bodies.map(bodyPreamble).filter(Boolean);
  const names: string[] = [...REQUIRED_SECTIONS];
  for (const sections of parsed) {
    for (const name of sections.keys()) {
      if (!names.includes(name)) names.push(name);
    }
  }
  const merged = names
    .map((name) => {
      const texts = parsed.map((sections) => sections.get(name)?.trim() ?? '').filter(Boolean);
      return texts.length ? `## ${name}\n\n${texts.join('\n\n')}\n` : `## ${name}\n`;
    })
    .join('\n');
  return preambles.length ? `${preambles.join('\n\n')}\n\n${merged}` : merged;
}

/** The text before a body's first level-1 or level-2 heading, trimmed. */
export function bodyPreamble(body: string): string {
  const lines = bodyLineTexts(body);
  const first = bodyHeadings(body)[0]?.index ?? lines.length;
  return trimBlankLines(lines.slice(0, first)).join('\n');
}

function trimBlankLines(lines: string[]): string[] {
  let start = 0;
  let end = lines.length;
  while (start < end && !(lines[start] ?? '').trim()) start += 1;
  while (end > start && !(lines[end - 1] ?? '').trim()) end -= 1;
  return lines.slice(start, end);
}
