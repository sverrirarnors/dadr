<script lang="ts">
  import type { TaskPackage } from '$lib/api';
  import { Badge } from '$lib/components/ui';
  import { PaneHeader, WorkbenchPane } from '$lib/components/workbench';
  import { outputKindLabel, taskIntentFor, taskIntents } from '$lib/tasks/presentation';

  type Props = {
    selectedTask: TaskPackage | null;
    taskCount: number;
    loading: boolean;
    contextChips: string[];
    activeTaskId?: string;
  };

  let { selectedTask, taskCount, loading, contextChips, activeTaskId = '' }: Props = $props();

  function taskIntentLabel(task: TaskPackage): string {
    return taskIntents.find((intent) => intent.id === taskIntentFor(task))?.label ?? 'Task';
  }
</script>

<WorkbenchPane data-task-workflow-pane class="min-h-0 overflow-auto">
  <PaneHeader
    title="Task workflow"
    subtitle={selectedTask ? selectedTask.manifest.title : `${taskCount} available tasks`}
  />
  <div class="space-y-3 p-3 text-sm">
    {#if selectedTask}
      <div class="space-y-2">
        <div class="flex flex-wrap items-center gap-1.5">
          <Badge>{selectedTask.source.kind}</Badge>
          <Badge>{taskIntentLabel(selectedTask)}</Badge>
          <Badge>{outputKindLabel(selectedTask.manifest.output.kind)}</Badge>
        </div>
      </div>
    {:else if loading}
      <p class="text-fg-muted">Loading tasks...</p>
    {:else}
      <p class="text-fg-muted">Choose a task to configure a run.</p>
    {/if}

    <section class="rounded-md border border-border bg-bg p-3" aria-label="Workflow context summary">
      <div class="text-xs font-medium uppercase text-fg-subtle">Context</div>
      <div class="mt-2 flex flex-wrap gap-1.5">
        {#each contextChips as chip (chip)}
          <Badge>{chip}</Badge>
        {/each}
      </div>
    </section>

    {#if activeTaskId}
      <section class="rounded-md border border-accent/30 bg-accent/5 p-3" aria-label="Active task">
        <div class="text-xs font-medium uppercase text-accent">Active run</div>
        <p class="mt-1 text-sm leading-5 text-fg-muted">{activeTaskId}</p>
      </section>
    {/if}
  </div>
</WorkbenchPane>
