import { describe, expect, it } from 'vitest';
import {
  Alert,
  Badge,
  Button,
  DisclosureRow,
  EmptyState,
  Field,
  InlineLinkBadge,
  KeyValueGrid,
  ListRow,
  PanelSection,
  SegmentedTabs,
  SelectInput,
  StackedCard,
  StatusPanel,
  TextareaInput,
  TextInput,
} from '../../src/lib/components/ui';
import {
  ActionBar,
  DocumentPane,
  InspectorPane,
  MetadataStrip,
  PaneHeader,
  PaneToolbar,
  WorkbenchPage,
  WorkbenchPane,
} from '../../src/lib/components/workbench';

describe('workbench primitive exports', () => {
  it('exports the UI atoms and workbench layout primitives', () => {
    expect(Button).toBeTruthy();
    expect(Badge).toBeTruthy();
    expect(Alert).toBeTruthy();
    expect(EmptyState).toBeTruthy();
    expect(DisclosureRow).toBeTruthy();
    expect(ListRow).toBeTruthy();
    expect(Field).toBeTruthy();
    expect(SegmentedTabs).toBeTruthy();
    expect(StatusPanel).toBeTruthy();
    expect(KeyValueGrid).toBeTruthy();
    expect(InlineLinkBadge).toBeTruthy();
    expect(PanelSection).toBeTruthy();
    expect(TextInput).toBeTruthy();
    expect(SelectInput).toBeTruthy();
    expect(TextareaInput).toBeTruthy();
    expect(StackedCard).toBeTruthy();
    expect(WorkbenchPage).toBeTruthy();
    expect(WorkbenchPane).toBeTruthy();
    expect(PaneHeader).toBeTruthy();
    expect(PaneToolbar).toBeTruthy();
    expect(DocumentPane).toBeTruthy();
    expect(InspectorPane).toBeTruthy();
    expect(MetadataStrip).toBeTruthy();
    expect(ActionBar).toBeTruthy();
  });
});
