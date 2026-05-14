function labelForKey(key: string): string {
  const label = key
    .split('_')
    .filter(Boolean)
    .join(' ');
  return label
    ? label.charAt(0).toUpperCase() + label.slice(1)
    : '';
}

function scalarSummaryValue(value: unknown): string {
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return '';
}

function nestedSummaryLines(value: Record<string, unknown>): string[] {
  return Object.entries(value).flatMap(([key, item]) => {
    const scalar = scalarSummaryValue(item);
    return scalar ? [`  - ${labelForKey(key)}: ${scalar}`] : [];
  });
}

export function taskSummaryMarkdown(value: unknown): string {
  if (typeof value === 'string') return value;
  if (!value || typeof value !== 'object' || Array.isArray(value)) return '';

  const lines: string[] = [];
  for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
    const label = labelForKey(key);
    const scalar = scalarSummaryValue(item);
    if (scalar) {
      lines.push(`- ${label}: ${scalar}`);
    } else if (Array.isArray(item)) {
      const entries = item.map((entry) => scalarSummaryValue(entry)).filter(Boolean);
      if (entries.length) {
        lines.push(`- ${label}:`);
        lines.push(...entries.map((entry) => `  - ${entry}`));
      }
    } else if (item && typeof item === 'object') {
      const nested = nestedSummaryLines(item as Record<string, unknown>);
      if (nested.length) {
        lines.push(`- ${label}:`);
        lines.push(...nested);
      }
    }
  }

  return lines.join('\n');
}
