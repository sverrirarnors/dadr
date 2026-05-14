<script lang="ts">
  import { Play } from 'lucide-svelte';
  import { Button, PanelSection, TextareaInput } from '$lib/components/ui';

  type Props = {
    title: string;
    instructions: string;
    running: boolean;
    onCancel: () => void;
    onRun: () => void;
  };

  let { title, instructions = $bindable(), running, onCancel, onRun }: Props = $props();
</script>

<PanelSection data-task-follow-up-composer class="mt-4 bg-bg">
  <h3 class="text-base font-semibold">Follow-up: {title}</h3>
  <label class="mt-3 block">
    <span class="mb-1 block text-sm font-medium text-fg">Follow-up instructions</span>
    <TextareaInput class="min-h-24 bg-surface" bind:value={instructions} />
  </label>
  <div class="mt-3 flex justify-end gap-2">
    <Button type="button" variant="outline" onclick={onCancel}>Cancel</Button>
    <Button type="button" disabled={running} onclick={onRun}>
      <Play class="h-3.5 w-3.5" />
      <span>{running ? 'Running' : 'Run follow-up'}</span>
    </Button>
  </div>
</PanelSection>
