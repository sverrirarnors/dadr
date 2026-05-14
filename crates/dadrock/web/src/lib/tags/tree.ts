import type { TagSummary } from '$lib/api';

export interface TagTreeNode {
  label: string;
  pathTag: string;
  directRecordCount: number;
  directOccurrenceCount: number;
  aggregateRecordCount: number;
  aggregateOccurrenceCount: number;
  children: TagTreeNode[];
}

export interface TagTreeRow extends TagTreeNode {
  depth: number;
}

function emptyNode(label: string, pathTag: string): TagTreeNode {
  return {
    label,
    pathTag,
    directRecordCount: 0,
    directOccurrenceCount: 0,
    aggregateRecordCount: 0,
    aggregateOccurrenceCount: 0,
    children: [],
  };
}

function tagSegments(tag: TagSummary): string[] {
  const value = (tag.display || tag.tag).replace(/^#/, '');
  return value
    .split('/')
    .map((segment) => segment.trim())
    .filter(Boolean);
}

function sortNodes(nodes: TagTreeNode[]): TagTreeNode[] {
  return nodes
    .sort((left, right) => left.label.localeCompare(right.label))
    .map((node) => {
      node.children = sortNodes(node.children);
      return node;
    });
}

function computeAggregates(node: TagTreeNode): TagTreeNode {
  let aggregateRecordCount = node.directRecordCount;
  let aggregateOccurrenceCount = node.directOccurrenceCount;
  for (const child of node.children) {
    computeAggregates(child);
    aggregateRecordCount += child.aggregateRecordCount;
    aggregateOccurrenceCount += child.aggregateOccurrenceCount;
  }
  node.aggregateRecordCount = aggregateRecordCount;
  node.aggregateOccurrenceCount = aggregateOccurrenceCount;
  return node;
}

export function buildTagTree(tags: TagSummary[]): TagTreeNode[] {
  const roots: TagTreeNode[] = [];

  for (const tag of tags) {
    const segments = tagSegments(tag);
    let siblings = roots;
    let current: TagTreeNode | undefined;

    for (let index = 0; index < segments.length; index += 1) {
      const label = segments[index]!;
      const pathTag = `#${segments.slice(0, index + 1).join('/')}`;
      current = siblings.find((node) => node.label === label);
      if (!current) {
        current = emptyNode(label, pathTag);
        siblings.push(current);
      }
      siblings = current.children;
    }

    if (current) {
      current.directRecordCount = tag.record_count;
      current.directOccurrenceCount = tag.occurrence_count;
    }
  }

  return sortNodes(roots).map(computeAggregates);
}

export function flattenTagTree(nodes: TagTreeNode[], depth = 0): TagTreeRow[] {
  return nodes.flatMap((node) => [
    { ...node, depth },
    ...flattenTagTree(node.children, depth + 1),
  ]);
}

export function tagRecordsHref(recordsHref: string, tag: string): string {
  return `${recordsHref}?q=${encodeURIComponent(tag)}`;
}

export function formatTagCount(node: TagTreeNode): string {
  const recordLabel = node.aggregateRecordCount === 1 ? 'record' : 'records';
  const useLabel = node.aggregateOccurrenceCount === 1 ? 'use' : 'uses';
  if (node.aggregateOccurrenceCount > node.aggregateRecordCount) {
    return `${node.aggregateRecordCount} ${recordLabel}, ${node.aggregateOccurrenceCount} ${useLabel}`;
  }
  return `${node.aggregateRecordCount} ${recordLabel}`;
}
