import { describe, expect, it } from 'vitest';
import { taskSummaryMarkdown } from '../../src/lib/tasks/result-summary';

describe('taskSummaryMarkdown', () => {
  it('formats object summaries without exposing raw JSON', () => {
    const markdown = taskSummaryMarkdown({
      by_status: {
        accepted: 5,
        candidate: 20,
        rejected: 2,
      },
      decisions_dir: '/Users/vincent/repos/harvest_articles/dadr/accepted',
      issue_count: 6,
      top_concerns: [
        '9 broken markdown links caused by wrong relative-path depth.',
        'Rejected records carry no rejection rationale.',
      ],
      total_records: 27,
      validation_errors: 9,
    });

    expect(markdown).toContain('Total records: 27');
    expect(markdown).toContain('Issue count: 6');
    expect(markdown).toContain('Accepted: 5');
    expect(markdown).toContain('9 broken markdown links');
    expect(markdown).not.toContain('by_status');
    expect(markdown).not.toContain('```json');
    expect(markdown).not.toContain('{');
  });
});
