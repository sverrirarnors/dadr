import { describe, expect, it } from 'vitest';
import type { RecordSummary } from '../../src/lib/api';
import {
  buildCommandPaletteItems,
  filterCommandPaletteItems,
} from '../../src/lib/command-palette/items';
import { primaryNavItems } from '../../src/lib/navigation/routes';

const records: RecordSummary[] = [
  {
    id: '0193fb1f-a3a0-7000-8000-000000000001',
    id_prefix: '0193fb1fa3a0',
    simple_record_id: '0193fb1fa3a070008000000000000001',
    title: 'Use Polars over pandas',
    status: 'accepted',
    superseded: false,
    path: 'dadr/ledger/0001-use-polars.md',
    preview: 'Use Polars for tabular transforms.',
    tags: ['#dadr/data/processing'],
    timestamp_ms: 1730000000000,
  },
];

describe('buildCommandPaletteItems', () => {
  it('includes navigation, record, and action items', () => {
    const items = buildCommandPaletteItems(records);

    expect(items.some((item) => item.label === 'Go to Ledger' && item.section === 'Navigation')).toBe(true);
    expect(
      items.some(
        (item) =>
          item.label === 'Use Polars over pandas' &&
          item.section === 'Records' &&
          item.recordId === '0193fb1f-a3a0-7000-8000-000000000001',
      ),
    ).toBe(true);
    expect(items.filter((item) => item.section === 'Actions').map((item) => item.label)).toEqual([
      'Toggle Theme',
      'Reload',
    ]);
    expect(items.some((item) => item.label === 'Go to Draft' && item.route === '/draft')).toBe(true);
  });

  it('builds navigation commands from the primary navigation model', () => {
    const items = buildCommandPaletteItems([]);

    expect(
      items
        .filter((item) => item.section === 'Navigation')
        .map((item) => ({ label: item.label, route: item.route })),
    ).toEqual(primaryNavItems.map((item) => ({ label: `Go to ${item.label}`, route: item.href })));
  });
});

describe('filterCommandPaletteItems', () => {
  it('matches records by title, path, id, and tag', () => {
    const items = buildCommandPaletteItems(records);

    for (const query of ['polars', '0001-use-polars', '0193fb1f', '#dadr/data']) {
      expect(filterCommandPaletteItems(items, query).map((item) => item.label)).toContain(
        'Use Polars over pandas',
      );
    }
  });
});
