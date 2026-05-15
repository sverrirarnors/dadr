<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { onDestroy } from 'svelte';
  import { Check, CircleStop, FilePlus2, Play, RefreshCw } from 'lucide-svelte';
  import {
    createApi,
    type MutationResponse,
    type RecordSummary,
    type TaskContext,
    type TaskInput,
    type TaskInputValue,
    type TaskPackage,
    type TaskResult,
    type TaskRun,
    type TaskRunLogEntry,
  } from '$lib/api';
  import { candidateDestination } from '$lib/candidates/form';
  import RecordPicker from '$lib/components/records/RecordPicker.svelte';
  import TaskFollowUpComposer from '$lib/components/tasks/TaskFollowUpComposer.svelte';
  import TaskHistoryPane from '$lib/components/tasks/TaskHistoryPane.svelte';
  import TaskPathPicker from '$lib/components/tasks/TaskPathPicker.svelte';
  import TaskResultsPanel from '$lib/components/tasks/TaskResultsPanel.svelte';
  import TaskRunnerPane from '$lib/components/tasks/TaskRunnerPane.svelte';
  import { Button, SegmentedTabs, StatusPanel } from '$lib/components/ui';
  import { ResizableWorkbenchSplit, WorkbenchPage, WorkbenchPane } from '$lib/components/workbench';
  import { renderMarkdown } from '$lib/markdown/render';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import {
    displayRecordStatus,
    type RecordStatusFilter,
  } from '$lib/records/filters';
  import {
    defaultRecordStatusForTask,
    outputKindLabel,
    taskRequirementTabLabel,
  } from '$lib/tasks/presentation';
  import {
    lines,
    objectArrayField,
    shortId,
    stringArrayField,
    stringField,
  } from '$lib/tasks/result-fields';
  import { taskSummaryMarkdown } from '$lib/tasks/result-summary';
  import { toasts } from '$lib/stores/toast.svelte';

  interface CandidateProposal {
    id?: string;
    title: string;
    tags: string[];
    body_markdown: string;
    confidence?: string;
    warnings: string[];
  }

  interface FollowUpTarget {
    title: string;
    task_id: string;
    input: Record<string, TaskInputValue>;
    context: TaskContext;
  }

  interface TaskPrefill {
    recordId: string;
    instructions: string;
    path: string;
  }

  interface LoadTaskResultOptions {
    writeSupersessionCandidate?: boolean;
  }

  type TaskTab = 'tasks' | 'runs';
  type ContextSection = 'details' | 'files' | 'records';

  const tabs: Array<{ value: TaskTab; label: string }> = [
    { value: 'tasks', label: 'Context' },
    { value: 'runs', label: 'Results' },
  ];

  const api = createApi();
  const queryClient = useQueryClient();

  let selectedTaskId = $state('');
  let selectedRunId = $state('');
  let appliedPrefillKey = $state('');
  let inputValues = $state<Record<string, TaskInputValue>>({});
  let taskContext = $state<Required<TaskContext>>({
    system_prompt: '',
    instructions: '',
    paths: [],
    records: [],
    text: '',
  });
  let activeContextSection = $state<ContextSection>('details');
  let fileContextText = $state('');
  let filePathPickerQuery = $state('');
  let directoryContextText = $state('');
  let directoryPathPickerQuery = $state('');
  let recordContextText = $state('');
  let recordSearchQuery = $state('');
  let recordStatusFilter = $state<RecordStatusFilter>('all');
  let taskResult = $state<TaskResult | null>(null);
  let taskResultRunId = $state('');
  let taskResultError = $state('');
  let taskResultLoading = $state(false);
  let draftBody = $state('');
  let revisionTitle = $state('');
  let revisionTags = $state('');
  let revisionBody = $state('');
  let revisionStateKey = $state('');
  let runProgress = $state('Ready to run a task.');
  let activeRunId = $state('');
  let taskLogOpen = $state(false);
  let followUpTarget = $state<FollowUpTarget | null>(null);
  let followUpInstructions = $state('');
  let dismissedResultItems = $state<string[]>([]);
  let activeRunPoll: number | null = null;

  const tasksQuery = createQuery(() => ({
    queryKey: ['tasks'],
    queryFn: () => api.listTasks(),
  }));

  const runsQuery = createQuery(() => ({
    queryKey: ['task-runs'],
    queryFn: () => api.listTaskRuns(),
    refetchInterval: activeRunId ? 1000 : false,
  }));

  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  const tasks = $derived(tasksQuery.data?.tasks ?? []);
  const records = $derived(snapshot.data?.records ?? []);
  const taskDiagnostics = $derived(tasksQuery.data?.diagnostics ?? []);
  const runs = $derived([...(runsQuery.data ?? [])].reverse());
  const selectedTask = $derived(tasks.find((task) => task.manifest.id === selectedTaskId) ?? null);
  const selectedRun = $derived(runs.find((run) => run.id === selectedRunId) ?? null);
  let selectedTab = $state<TaskTab>(readTab(page.url.searchParams.get('tab')));
  const activeRun = $derived(runs.find((run) => run.id === activeRunId) ?? null);
  const taskLogRunId = $derived(activeRunId || selectedRunId || taskResultRunId);
  const resultRunLabel = $derived(
    taskResultRunId ? shortId(taskResultRunId) : selectedRun ? shortId(selectedRun.id) : 'No run selected',
  );
  const sourceRecordId = $derived(page.url.searchParams.get('record') ?? '');
  const sourceRecord = $derived(recordById(sourceRecordId));
  const selectedContextFilePaths = $derived(unique(lines(fileContextText)));
  const selectedContextDirectoryPaths = $derived(unique(lines(directoryContextText)));
  const selectedContextPaths = $derived(
    unique([...selectedContextFilePaths, ...selectedContextDirectoryPaths]),
  );
  const selectedContextRecordIds = $derived(unique(lines(recordContextText)));
  const taskRequirementState = $derived({
    recordCount: selectedContextRecordIds.length,
    pathCount: selectedContextPaths.length,
    hasNotes: Boolean(taskContext.instructions.trim()),
  });
  const taskLogQuery = createQuery(() => ({
    queryKey: ['task-run-log', taskLogRunId],
    queryFn: () => api.getTaskLog(taskLogRunId),
    enabled: Boolean(taskLogRunId),
    refetchInterval: activeRunId ? 1000 : false,
  }));

  const taskLog = $derived(taskLogQuery.data ?? []);

  onDestroy(stopActiveRunPolling);

  $effect(() => {
    if (!tasks.length) {
      selectedTaskId = '';
      return;
    }
    if (!tasks.some((task) => task.manifest.id === selectedTaskId)) {
      const firstTask = tasks[0];
      if (firstTask) {
        selectTask(firstTask.manifest.id);
      }
    }
  });

  $effect(() => {
    if (!tasks.length) return;
    const taskId = page.url.searchParams.get('task') ?? '';
    const recordId = page.url.searchParams.get('record') ?? '';
    const instructions = page.url.searchParams.get('instructions') ?? '';
    const path = page.url.searchParams.get('path') ?? '';
    const key = JSON.stringify([taskId, recordId, instructions, path]);
    if (!taskId || appliedPrefillKey === key) return;
    const task = tasks.find((candidate) => candidate.manifest.id === taskId);
    if (!task) return;
    applyTaskPrefill(task, { recordId, instructions, path });
    appliedPrefillKey = key;
  });

  $effect(() => {
    if (taskResult?.kind === 'candidate_draft.v1') {
      draftBody = stringField(taskResult, 'body_markdown');
    }
  });

  $effect(() => {
    if (taskResult?.kind !== 'record_revision.v1') {
      revisionStateKey = '';
      revisionTitle = '';
      revisionTags = '';
      revisionBody = '';
      return;
    }
    const recordId = stringField(taskResult, 'record_id');
    const key = `${taskResultRunId}:${recordId}`;
    if (revisionStateKey === key) return;
    const record = recordById(recordId);
    revisionTitle = stringField(taskResult, 'title') || record?.title || '';
    const resultTags = stringArrayField(taskResult, 'tags');
    revisionTags = (resultTags.length ? resultTags : (record?.tags ?? [])).join('\n');
    revisionBody = stringField(taskResult, 'body_markdown');
    revisionStateKey = key;
  });

  $effect(() => {
    if (!activeRunId || !activeRun) return;
    applyActiveRunStatus(activeRun);
  });

  const runTask = createMutation(() => ({
    mutationFn: () => {
      const request = {
        task_id: selectedTaskId,
        input: taskRunInput(selectedTask),
      };
      const context = taskRunContext();
      return api.runTask(context ? { ...request, context } : request);
    },
    onMutate: () => {
      runProgress = selectedTask
        ? `Running ${selectedTask.manifest.title}. Waiting for the local agent...`
        : 'Running task. Waiting for the local agent...';
      taskLogOpen = false;
      activeRunId = '';
      setTab('tasks');
    },
    onSuccess: async (run) => {
      selectedRunId = run.id;
      taskLogOpen = false;
      await queryClient.invalidateQueries({ queryKey: ['task-runs'] });
      await queryClient.invalidateQueries({ queryKey: ['task-run-log', run.id] });
      if (run.status === 'succeeded') {
        runProgress = `Task ${run.task_id} finished with status ${run.status}.`;
        toasts.push({ severity: 'success', message: `task run ${run.status}` });
        await loadTaskResult(run.id, { writeSupersessionCandidate: run.task_id === 'supersede-record' });
      } else if (run.status === 'failed') {
        runProgress = `Task ${run.task_id} failed.`;
        toasts.push({ severity: 'error', message: 'Task run failed' });
        taskResult = null;
        taskResultRunId = run.id;
        taskResultError = 'Task run failed.';
        setTab('runs');
      } else {
        activeRunId = run.id;
        runProgress = `Task ${run.task_id} started. Agent output is updating.`;
        startActiveRunPolling(run.id);
      }
    },
    onError: (error) => {
      runProgress = `Task failed: ${error.message}`;
      toasts.push({ severity: 'error', message: 'Task run failed', detail: error.message });
    },
  }));

  const cancelTask = createMutation(() => ({
    mutationFn: (runId: string) => api.cancelTaskRun(runId),
    onSuccess: async (run) => {
      activeRunId = '';
      stopActiveRunPolling();
      selectedRunId = run.id;
      taskResult = null;
      taskResultRunId = run.id;
      taskResultError = '';
      taskLogOpen = false;
      runProgress = `Task ${run.task_id} cancelled.`;
      toasts.push({ severity: 'info', message: 'Task cancelled' });
      await queryClient.invalidateQueries({ queryKey: ['task-runs'] });
      await queryClient.invalidateQueries({ queryKey: ['task-run-log', run.id] });
    },
    onError: (error) => {
      toasts.push({ severity: 'error', message: 'Task cancel failed', detail: error.message });
    },
  }));

  const clearTaskHistory = createMutation(() => ({
    mutationFn: () => api.clearTaskHistory(),
    onSuccess: async (response) => {
      activeRunId = '';
      stopActiveRunPolling();
      selectedRunId = '';
      taskResult = null;
      taskResultRunId = '';
      taskResultError = '';
      taskLogOpen = false;
      runProgress = response.message;
      queryClient.setQueryData<TaskRun[]>(['task-runs'], []);
      toasts.push({ severity: 'success', message: response.message });
      await Promise.all([
        applyMutationResponse(queryClient, response),
        queryClient.invalidateQueries({ queryKey: ['task-runs'] }),
      ]);
    },
    onError: (error) => {
      toasts.push({ severity: 'error', message: 'Clear history failed', detail: error.message });
    },
  }));

  const runFollowUp = createMutation(() => ({
    mutationFn: () => {
      if (!followUpTarget) {
        throw new Error('No follow-up target selected');
      }
      return api.runTask({
        task_id: followUpTarget.task_id,
        input: followUpInput(followUpTarget, followUpInstructions),
        context: followUpContext(followUpTarget, followUpInstructions),
      });
    },
    onMutate: () => {
      runProgress = followUpTarget
        ? `Running follow-up for ${followUpTarget.title}.`
        : 'Running follow-up.';
      taskLogOpen = false;
      activeRunId = '';
    },
    onSuccess: async (run) => {
      followUpTarget = null;
      followUpInstructions = '';
      selectedRunId = run.id;
      taskLogOpen = false;
      await queryClient.invalidateQueries({ queryKey: ['task-runs'] });
      await queryClient.invalidateQueries({ queryKey: ['task-run-log', run.id] });
      if (run.status === 'succeeded') {
        runProgress = `Follow-up ${run.task_id} finished with status ${run.status}.`;
        toasts.push({ severity: 'success', message: `follow-up ${run.status}` });
        await loadTaskResult(run.id, { writeSupersessionCandidate: run.task_id === 'supersede-record' });
      } else {
        activeRunId = run.id;
        runProgress = `Follow-up ${run.task_id} started. Agent output is updating.`;
        startActiveRunPolling(run.id);
      }
    },
    onError: (error) => {
      toasts.push({ severity: 'error', message: 'Follow-up failed', detail: error.message });
    },
  }));

  const writeCandidate = createMutation(() => ({
    mutationFn: (candidate: CandidateProposal) =>
      api.createCandidate({
        title: candidate.title,
        tags: candidate.tags,
        body_markdown: candidate.body_markdown,
      }),
    onSuccess: finishCandidate,
    onError: (error) => {
      toasts.push({ severity: 'error', message: 'Candidate write failed', detail: error.message });
    },
  }));

  const applyRecordRevision = createMutation(() => ({
    mutationFn: () => {
      if (!taskResult || taskResult.kind !== 'record_revision.v1') {
        throw new Error('No record revision selected');
      }
      const recordId = stringField(taskResult, 'record_id');
      if (!recordId) {
        throw new Error('Record revision is missing record_id');
      }
      return api.applyRecordRevision({
        record_id: recordId,
        title: revisionTitle.trim(),
        tags: lines(revisionTags),
        body_markdown: revisionBody,
      });
    },
    onSuccess: finishRecordRevision,
    onError: (error) => {
      toasts.push({ severity: 'error', message: 'Revision apply failed', detail: error.message });
    },
  }));

  const busy = $derived(
    tasksQuery.isPending ||
      runsQuery.isPending ||
      runTask.isPending ||
      Boolean(activeRunId) ||
      cancelTask.isPending ||
      clearTaskHistory.isPending ||
      runFollowUp.isPending ||
      writeCandidate.isPending ||
      applyRecordRevision.isPending ||
      taskResultLoading,
  );
  const canRun = $derived(Boolean(selectedTask) && requiredInputsSatisfied(selectedTask) && !busy);

  function readTab(value: string | null): TaskTab {
    return tabs.some((tab) => tab.value === value) ? (value as TaskTab) : 'tasks';
  }

  function setTab(tab: TaskTab) {
    selectedTab = tab;
    const next = new URL(page.url);
    if (tab === 'tasks') {
      next.searchParams.delete('tab');
    } else {
      next.searchParams.set('tab', tab);
    }
    const target = (next.search ? `/tasks${next.search}` : '/tasks') as
      | '/tasks'
      | `/tasks?${string}`;
    void goto(resolve(target), {
      replaceState: true,
      noScroll: true,
      keepFocus: true,
    });
  }

  function startActiveRunPolling(runId: string) {
    stopActiveRunPolling();
    void refreshActiveRun(runId);
    activeRunPoll = window.setInterval(() => {
      void refreshActiveRun(runId);
    }, 1000);
  }

  function stopActiveRunPolling() {
    if (activeRunPoll !== null) {
      window.clearInterval(activeRunPoll);
      activeRunPoll = null;
    }
  }

  async function refreshActiveRun(runId: string) {
    try {
      const latestRuns = await api.listTaskRuns();
      queryClient.setQueryData(['task-runs'], latestRuns);
      const latestRun = latestRuns.find((run) => run.id === runId);
      if (latestRun) {
        applyActiveRunStatus(latestRun);
      }
      const latestLog = await api.getTaskLog(runId);
      queryClient.setQueryData(['task-run-log', runId], latestLog);
    } catch (error) {
      toasts.push({
        severity: 'error',
        message: 'Task status refresh failed',
        detail: error instanceof Error ? error.message : String(error),
      });
    }
  }

  function applyActiveRunStatus(run: TaskRun) {
    if (run.id !== activeRunId) return;
    if (run.status === 'created' || run.status === 'running') {
      runProgress = `${inProgressRunMessage(run)} Agent output is updating.`;
      return;
    }
    activeRunId = '';
    stopActiveRunPolling();
    if (run.status === 'succeeded') {
      void loadTaskResult(run.id, { writeSupersessionCandidate: run.task_id === 'supersede-record' });
    } else if (run.status === 'cancelled') {
      taskResult = null;
      taskResultRunId = run.id;
      taskResultError = '';
      runProgress = `Task ${run.task_id} cancelled.`;
      setTab('tasks');
    } else {
      taskResult = null;
      taskResultRunId = run.id;
      taskResultError = 'Task run failed.';
      runProgress = `Task ${run.task_id} failed.`;
      setTab('runs');
    }
  }

  async function finishCandidate(response: MutationResponse) {
    toasts.push({ severity: 'success', message: response.message });
    await applyMutationResponse(queryClient, response);
    const destination = candidateDestination(response);
    if (destination.params) {
      await goto(resolve('/records/[id]', destination.params));
    } else {
      await goto(resolve('/records'));
    }
  }

  async function finishRecordRevision(response: MutationResponse) {
    toasts.push({ severity: 'success', message: response.message });
    const selectedId = response.focus?.selected_id ?? (taskResult ? stringField(taskResult, 'record_id') : '');
    await applyMutationResponse(queryClient, response);
    if (selectedId) {
      await goto(resolve('/records/[id]', { id: selectedId }));
    } else {
      await goto(resolve('/records'));
    }
  }

  function selectTask(taskId: string) {
    const task = tasks.find((candidate) => candidate.manifest.id === taskId);
    selectedTaskId = taskId;
    inputValues = task ? defaultInputValues(task) : {};
    taskContext = emptyTaskContext(task);
    activeContextSection = 'details';
    fileContextText = '';
    filePathPickerQuery = '';
    directoryContextText = '';
    directoryPathPickerQuery = '';
    recordContextText = '';
    recordSearchQuery = '';
    recordStatusFilter = defaultRecordStatusForTask(taskId);
    clearTaskResult();
  }

  function applyTaskPrefill(task: TaskPackage, prefill: TaskPrefill) {
    selectedTaskId = task.manifest.id;
    activeContextSection = 'details';
    const nextInput = defaultInputValues(task);
    if (prefill.recordId) {
      for (const input of task.manifest.inputs) {
        if (input.name === 'record_id') {
          nextInput[input.name] = prefill.recordId;
        }
      }
    }
    inputValues = nextInput;
    taskContext = { ...emptyTaskContext(task), instructions: prefill.instructions };
    fileContextText = prefill.path;
    filePathPickerQuery = '';
    directoryContextText = '';
    directoryPathPickerQuery = '';
    recordContextText = prefill.recordId;
    recordSearchQuery = '';
    recordStatusFilter = defaultRecordStatusForTask(task.manifest.id);
    activeContextSection = 'details';
    clearTaskResult();
  }

  function emptyTaskContext(task?: TaskPackage | null): Required<TaskContext> {
    return {
      system_prompt: task?.prompt ?? '',
      instructions: '',
      paths: [],
      records: [],
      text: '',
    };
  }

  function defaultInputValues(task: TaskPackage): Record<string, TaskInputValue> {
    return Object.fromEntries(
      task.manifest.inputs.map((input) => [input.name, defaultInputValue(input)]),
    );
  }

  function defaultInputValue(input: TaskInput): TaskInputValue {
    if (input.type === 'checkbox') return String(input.default ?? '').toLowerCase() === 'true';
    if (input.type === 'multiselect') return [];
    if (input.type === 'number') return input.default === undefined ? '' : Number(input.default);
    return input.default ?? '';
  }

  function taskRunInput(task: TaskPackage | null): Record<string, TaskInputValue> {
    if (!task) return {};
    const output: Record<string, TaskInputValue> = {};
    const primaryDetail = primaryDetailInput(task);
    for (const input of task.manifest.inputs) {
      const value = taskInputRunValue(input, primaryDetail);
      if (!input.required && isEmptyInput(value)) continue;
      output[input.name] = value;
    }
    return output;
  }

  function taskRunContext(): TaskContext | undefined {
    const records = lines(recordContextText);
    const context: TaskContext = {};
    if (taskContext.instructions.trim()) context.instructions = taskContext.instructions.trim();
    if (selectedContextPaths.length) context.paths = selectedContextPaths;
    if (records.length) context.records = records;
    return Object.keys(context).length ? context : undefined;
  }

  function unique(values: string[]): string[] {
    return Array.from(new Set(values));
  }

  function recordById(recordId: string): RecordSummary | undefined {
    if (!recordId) return undefined;
    return records.find(
      (record) =>
        record.id === recordId ||
        record.simple_record_id === recordId ||
        record.id_prefix === recordId,
    );
  }

  function recordHref(recordId: string): '/records' | `/records?${string}` {
    return recordId ? `/records?id=${encodeURIComponent(recordId)}` : '/records';
  }

  function addContextRecord(recordId: string) {
    const nextIds = unique([...selectedContextRecordIds, recordId]);
    recordContextText = nextIds.join('\n');
  }

  function removeContextRecord(recordId: string) {
    recordContextText = selectedContextRecordIds.filter((candidate) => candidate !== recordId).join('\n');
  }

  function addContextFilePath(path: string) {
    const trimmed = path.trim();
    if (!trimmed) return;
    fileContextText = unique([...selectedContextFilePaths, trimmed]).join('\n');
    filePathPickerQuery = '';
  }

  function removeContextFilePath(path: string) {
    fileContextText = selectedContextFilePaths.filter((candidate) => candidate !== path).join('\n');
  }

  function addContextDirectoryPath(path: string) {
    const trimmed = path.trim();
    if (!trimmed) return;
    directoryContextText = unique([...selectedContextDirectoryPaths, trimmed]).join('\n');
    directoryPathPickerQuery = '';
  }

  function removeContextDirectoryPath(path: string) {
    directoryContextText = selectedContextDirectoryPaths
      .filter((candidate) => candidate !== path)
      .join('\n');
  }

  function requiredInputsSatisfied(task: TaskPackage | null): boolean {
    if (!task) return false;
    const primaryDetail = primaryDetailInput(task);
    return task.manifest.inputs.every((input) => {
      const value = taskInputRunValue(input, primaryDetail);
      return !input.required || !isEmptyInput(value);
    });
  }

  function visibleTaskInputs(task: TaskPackage): TaskInput[] {
    return task.manifest.inputs.filter(
      (input) => !isRecordInput(input) && !isPathInput(input) && !isDetailInput(input) && !isScopeInput(input),
    );
  }

  function scopeTaskInputs(task: TaskPackage): TaskInput[] {
    return task.manifest.inputs.filter(isScopeInput);
  }

  function isScopeInput(input: TaskInput): boolean {
    return input.name === 'status_scope';
  }

  function isRecordInput(input: TaskInput): boolean {
    return input.name === 'record_id' || input.name === 'record_ids';
  }

  function recordInputValue(input: TaskInput): TaskInputValue {
    if (input.name === 'record_ids') return selectedContextRecordIds.join('\n');
    return selectedContextRecordIds[0] ?? '';
  }

  function isPathInput(input: TaskInput): boolean {
    return input.name === 'path' || input.name === 'path_or_diff';
  }

  function isDetailInput(input: TaskInput): boolean {
    return [
      'context',
      'discussion',
      'focus',
      'requested_changes',
      'reason',
      'merge_goal',
      'review_focus',
      'tag_goal',
      'decision_hint',
    ].includes(input.name);
  }

  function primaryDetailInput(task: TaskPackage): TaskInput | undefined {
    const details = task.manifest.inputs.filter(isDetailInput);
    return details.find((input) => input.required) ?? details[0];
  }

  function taskInputRunValue(input: TaskInput, primaryDetail?: TaskInput): TaskInputValue {
    if (isRecordInput(input)) return recordInputValue(input);
    if (isPathInput(input)) return pathInputValue(input);
    if (isDetailInput(input)) return detailInputValue(input, primaryDetail);
    return inputValues[input.name] ?? defaultInputValue(input);
  }

  function pathInputValue(input: TaskInput): TaskInputValue {
    if (input.name === 'path') {
      return selectedContextPaths[0] ?? inputValues[input.name] ?? defaultInputValue(input);
    }
    return selectedContextPaths.join('\n') || taskContext.instructions.trim();
  }

  function detailInputValue(input: TaskInput, primaryDetail?: TaskInput): TaskInputValue {
    if (primaryDetail?.name !== input.name) return inputValues[input.name] ?? defaultInputValue(input);
    const note = taskContext.instructions.trim();
    if (note) return note;
    if (input.name === 'context' || input.name === 'discussion') {
      return [
        ...selectedContextPaths.map((path) => `Path: ${path}`),
        ...selectedContextRecordIds.map((record) => `Record: ${record}`),
      ].join('\n');
    }
    return inputValues[input.name] ?? defaultInputValue(input);
  }

  function normalizedInputLabel(input: TaskInput): string {
    if (isScopeInput(input)) return 'Scope';
    return input.label;
  }

  function detailsRequired(task: TaskPackage): boolean {
    const primaryDetail = primaryDetailInput(task);
    if (!primaryDetail) return false;
    return primaryDetail.required;
  }

  function isEmptyInput(value: TaskInputValue | undefined): boolean {
    return value === undefined || value === '' || (Array.isArray(value) && value.length === 0);
  }

  function updateInput(input: TaskInput, event: Event) {
    const target = event.currentTarget as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement;
    if (input.type === 'checkbox' && target instanceof HTMLInputElement) {
      inputValues[input.name] = target.checked;
    } else if (input.type === 'multiselect' && target instanceof HTMLSelectElement) {
      inputValues[input.name] = Array.from(target.selectedOptions).map((option) => option.value);
    } else if (input.type === 'number' && target instanceof HTMLInputElement) {
      inputValues[input.name] = target.value === '' ? '' : Number(target.value);
    } else {
      inputValues[input.name] = target.value;
    }
  }

  async function loadTaskResult(runId: string, options: LoadTaskResultOptions = {}) {
    selectedRunId = runId;
    taskLogOpen = false;
    taskResult = null;
    taskResultRunId = runId;
    taskResultError = '';
    taskResultLoading = true;
    runProgress = `Loading result ${shortId(runId)}...`;
    setTab('runs');
    try {
      const result = await api.getTaskResult(runId);
      taskResult = result;
      dismissedResultItems = [];
      runProgress = `Loaded result ${shortId(runId)}.`;
      if (options.writeSupersessionCandidate && result.kind === 'supersession_proposal.v1') {
        writeSupersessionCandidateFromResult(result);
      }
    } catch (error) {
      taskResultError = error instanceof Error ? error.message : String(error);
      runProgress = `Result load failed: ${taskResultError}`;
      toasts.push({ severity: 'error', message: 'Task result failed', detail: taskResultError });
    } finally {
      taskResultLoading = false;
    }
  }

  function selectTaskRun(runId: string) {
    const run = runs.find((candidate) => candidate.id === runId);
    if (!run) {
      void loadTaskResult(runId);
      return;
    }
    if (run.status === 'succeeded') {
      void loadTaskResult(run.id);
      return;
    }
    selectedRunId = run.id;
    taskLogOpen = false;
    taskResult = null;
    taskResultRunId = run.id;
    taskResultError = '';
    taskResultLoading = false;
    setTab('runs');
    if (isRunInProgress(run)) {
      activeRunId = run.id;
      runProgress = inProgressRunMessage(run);
      startActiveRunPolling(run.id);
    } else if (run.status === 'cancelled') {
      if (activeRunId === run.id) {
        activeRunId = '';
        stopActiveRunPolling();
      }
      runProgress = `Task ${run.task_id} cancelled.`;
    } else {
      if (activeRunId === run.id) {
        activeRunId = '';
        stopActiveRunPolling();
      }
      taskResultError = 'Task run failed.';
      runProgress = `Task ${run.task_id} failed.`;
    }
  }

  function clearTaskResult() {
    selectedRunId = '';
    taskResult = null;
    taskResultRunId = '';
    taskResultError = '';
    taskResultLoading = false;
    draftBody = '';
    followUpTarget = null;
    followUpInstructions = '';
    dismissedResultItems = [];
    revisionTitle = '';
    revisionTags = '';
    revisionBody = '';
    revisionStateKey = '';
  }

  function resultItemKey(kind: string, item: Record<string, unknown>, index: number): string {
    return `${taskResultRunId}:${kind}:${String(item.id ?? item.record_id ?? index)}`;
  }

  function isDismissed(kind: string, item: Record<string, unknown>, index: number): boolean {
    return dismissedResultItems.includes(resultItemKey(kind, item, index));
  }

  function dismissResultItem(kind: string, item: Record<string, unknown>, index: number) {
    const key = resultItemKey(kind, item, index);
    if (!dismissedResultItems.includes(key)) {
      dismissedResultItems = [...dismissedResultItems, key];
    }
  }

  function startFollowUp(target: FollowUpTarget) {
    followUpTarget = target;
    followUpInstructions = '';
  }

  function followUpInput(target: FollowUpTarget, instructions: string): Record<string, TaskInputValue> {
    const input = { ...target.input };
    const note = instructions.trim();
    if (!note) return input;
    if (target.task_id === 'edit-record' && typeof input.requested_changes === 'string') {
      input.requested_changes = `${input.requested_changes}\n\nUser note: ${note}`;
    } else if (target.task_id === 'context-to-record' && typeof input.context === 'string') {
      input.context = `${input.context}\n\nUser note: ${note}`;
    }
    return input;
  }

  function followUpContext(target: FollowUpTarget, instructions: string): TaskContext {
    const note = instructions.trim();
    const context: TaskContext = {
      ...target.context,
    };
    const combinedInstructions = [target.context.instructions, note].filter(Boolean).join('\n\n');
    if (combinedInstructions) {
      context.instructions = combinedInstructions;
    }
    return context;
  }

  function candidateFollowUp(candidate: CandidateProposal): FollowUpTarget {
    return {
      title: candidate.title,
      task_id: 'context-to-record',
      input: {
        context: `${candidate.title}\n\n${candidate.body_markdown}`,
      },
      context: {
        text: JSON.stringify(candidate, null, 2),
      },
    };
  }

  function writeDraftCandidate() {
    if (!taskResult) return;
    writeCandidate.mutate({
      title: stringField(taskResult, 'title') || 'Task candidate',
      tags: stringArrayField(taskResult, 'tags'),
      body_markdown: draftBody || stringField(taskResult, 'body_markdown'),
      warnings: stringArrayField(taskResult, 'warnings'),
    });
  }

  function supersessionCandidateProposal(result: TaskResult): CandidateProposal {
    return {
      title: stringField(result, 'superseding_title') || 'Superseding candidate',
      tags: ['#writer/agent'],
      body_markdown: stringField(result, 'superseding_body_markdown'),
      warnings: [],
    };
  }

  function writeSupersessionCandidateFromResult(result: TaskResult) {
    const candidate = supersessionCandidateProposal(result);
    if (!candidate.body_markdown.trim()) return;
    writeCandidate.mutate(candidate);
  }

  function writeSupersessionCandidate() {
    if (!taskResult) return;
    writeSupersessionCandidateFromResult(taskResult);
  }

  function startRecordRevisionFollowUp() {
    if (!taskResult) return;
    const recordId = stringField(taskResult, 'record_id');
    startFollowUp({
      title: stringField(taskResult, 'summary') || 'Record revision',
      task_id: 'edit-record',
      input: {
        record_id: recordId,
        requested_changes: stringField(taskResult, 'change_explanation'),
      },
      context: {
        records: recordId ? [recordId] : [],
        text: stringField(taskResult, 'body_markdown'),
      },
    });
  }

  function startTaskForRecord(recordId: string) {
    const task =
      tasks.find((candidate) => candidate.manifest.id === 'edit-record') ??
      tasks.find((candidate) => candidate.manifest.id === selectedTaskId);
    if (task) {
      applyTaskPrefill(task, { recordId, instructions: '', path: '' });
    } else {
      recordContextText = recordId;
      activeContextSection = 'records';
    }
    setTab('tasks');
  }

  function summaryText(result: TaskResult): string {
    return taskSummaryMarkdown(result.summary);
  }

  function candidateArray(result: TaskResult): CandidateProposal[] {
    const value = result.candidates;
    if (!Array.isArray(value)) return [];
    return value.flatMap((item) => {
      if (!item || typeof item !== 'object') return [];
      const candidate = item as Record<string, unknown>;
      const title = typeof candidate.title === 'string' ? candidate.title : '';
      const body = typeof candidate.body_markdown === 'string' ? candidate.body_markdown : '';
      if (!title || !body) return [];
      const proposal: CandidateProposal = {
        title,
        body_markdown: body,
        tags: Array.isArray(candidate.tags)
          ? candidate.tags.filter((tag): tag is string => typeof tag === 'string')
          : [],
        warnings: Array.isArray(candidate.warnings)
          ? candidate.warnings.filter((warning): warning is string => typeof warning === 'string')
          : [],
      };
      if (typeof candidate.confidence === 'string') {
        proposal.confidence = candidate.confidence;
      }
      if (typeof candidate.id === 'string') {
        proposal.id = candidate.id;
      }
      return [proposal];
    });
  }

  function multiselectSelected(input: TaskInput, option: string): boolean {
    const value = inputValues[input.name];
    return Array.isArray(value) && value.includes(option);
  }

  function isRunInProgress(run: TaskRun): boolean {
    return run.status === 'created' || run.status === 'running';
  }

  function inProgressRunMessage(run: TaskRun): string {
    if (run.status === 'created') {
      return `Task ${run.task_id} has been created and is waiting to start.`;
    }
    return `Task ${run.task_id} is still running.`;
  }

  function logTime(entry: TaskRunLogEntry): string {
    const date = new Date(entry.timestamp);
    if (Number.isNaN(date.getTime())) return '';
    return date.toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    });
  }
</script>

<WorkbenchPage layout="single" class={selectedTab === 'runs' ? 'h-full' : ''}>
  <WorkbenchPane class={selectedTab === 'runs' ? 'min-h-0 overflow-hidden' : 'min-h-0 overflow-auto'}>
    <div class={selectedTab === 'runs' ? 'flex h-full min-h-0 flex-col p-3' : 'p-3'}>
  <div class={selectedTab === 'runs' ? 'flex min-h-0 flex-1 flex-col' : 'max-w-6xl'}>
    <h1 class="sr-only">Task</h1>

      <header class="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 class="text-2xl font-semibold">Task</h2>
          <p class="mt-1 text-sm text-fg-muted">
            {tasks.length} available task{tasks.length === 1 ? '' : 's'}
          </p>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          {#if activeRunId}
            <button
              type="button"
              class="inline-flex h-8 items-center gap-1.5 rounded-md border border-danger/40 bg-bg px-2.5 text-sm font-medium text-danger transition-colors hover:bg-danger/10 disabled:cursor-not-allowed disabled:opacity-50"
              disabled={cancelTask.isPending}
              onclick={() => cancelTask.mutate(activeRunId)}
            >
              <CircleStop class="h-3.5 w-3.5" />
              <span>{cancelTask.isPending ? 'Cancelling' : 'Cancel task'}</span>
            </button>
          {/if}
          <Button
            type="button"
            variant="outline"
            disabled={!canRun}
            onclick={() => runTask.mutate()}
          >
            <Play class="h-3.5 w-3.5" />
            <span>{runTask.isPending ? 'Running' : 'Run task'}</span>
          </Button>
        </div>
      </header>

    <SegmentedTabs items={tabs} value={selectedTab} ariaLabel="Task sections" onselect={setTab} />

    <StatusPanel
      class="mt-4"
      label="Task status"
      message={runProgress}
      busy={runTask.isPending || Boolean(activeRunId) || taskResultLoading || runsQuery.isFetching}
      disabled={!taskLogRunId}
      bind:open={taskLogOpen}
      detailLabel={taskLogRunId
        ? `${taskLogOpen ? 'Hide' : 'Show'} agent output · ${taskLog.length} line${taskLog.length === 1 ? '' : 's'}`
        : ''}
    >
        <div class="max-h-64 overflow-auto font-mono text-xs leading-5">
          {#if taskLogQuery.isPending}
            <div class="text-fg-muted">Loading agent output...</div>
          {:else if taskLogQuery.isError}
            <div class="text-danger">{taskLogQuery.error.message}</div>
          {:else if taskLog.length}
            <div class="space-y-1">
              {#each taskLog as entry, index (`${entry.timestamp}-${index}`)}
                <div class="grid gap-2 text-fg-muted sm:grid-cols-[5rem_1fr]">
                  <time class="whitespace-nowrap tabular-nums text-fg-subtle">{logTime(entry)}</time>
                  <span class="whitespace-pre-wrap break-words">{entry.message}</span>
                </div>
              {/each}
            </div>
          {:else}
            <div class="text-fg-muted">No agent output yet.</div>
          {/if}
        </div>
    </StatusPanel>

    {#if selectedTab === 'tasks' && sourceRecordId}
      <section
        aria-label="Task source record"
        class="mt-3 rounded-md border border-accent/30 bg-accent/5 p-3"
      >
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="text-xs font-medium uppercase text-accent">Working from record</div>
            {#if sourceRecord}
              <div class="mt-1 flex flex-wrap items-center gap-2">
                <h2 class="min-w-0 text-base font-semibold text-fg">{sourceRecord.title}</h2>
                <span class="rounded bg-surface px-1.5 py-0.5 text-[11px] text-fg-muted">
                  {displayRecordStatus(sourceRecord)}
                </span>
              </div>
              <div class="mt-1 truncate text-xs text-fg-subtle">{sourceRecord.path}</div>
            {:else}
              <div class="mt-1 font-mono text-sm text-fg">{sourceRecordId}</div>
            {/if}
          </div>
          <a
            class="inline-flex h-8 items-center rounded-md border border-border bg-bg px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface hover:text-fg"
            href={resolve(recordHref(sourceRecordId))}
          >
            Back to record
          </a>
        </div>
      </section>
    {/if}

    {#if selectedTab === 'tasks'}
      <TaskRunnerPane>
          {#if tasksQuery.isPending}
            <p class="text-sm text-fg-muted">Loading tasks...</p>
          {:else if tasksQuery.isError}
            <div class="rounded-md border border-danger/40 bg-danger/5 p-3 text-sm">
              <div class="font-medium text-danger">Failed to load tasks</div>
              <div class="mt-1 text-fg-muted">{tasksQuery.error.message}</div>
            </div>
          {:else if tasks.length && selectedTask}
            <div class="grid gap-5">
              <label class="block">
                <span class="mb-1 block text-sm font-medium text-fg">Task</span>
                <select
                  class="h-9 w-full rounded-md border border-border bg-bg px-3 text-sm text-fg focus:outline-none focus:ring-2 focus:ring-accent"
                  value={selectedTaskId}
                  onchange={(event) => selectTask(event.currentTarget.value)}
                >
                  {#each tasks as task (task.manifest.id)}
                    <option value={task.manifest.id}>{task.manifest.title}</option>
                  {/each}
                </select>
                <p data-task-description class="mt-2 text-sm leading-5 text-fg-muted">
                  {selectedTask.manifest.description}
                </p>
              </label>

              <section data-task-context-editor class="space-y-2">
                <h3 class="text-sm font-semibold text-fg">Context</h3>
                <div
                  data-task-context-layout
                  class="grid overflow-hidden rounded-md border border-border bg-bg md:grid-cols-[13rem_minmax(0,1fr)]"
                >
                  <div
                    data-task-context-left-pane
                    data-task-context-nav
                    role="tablist"
                    aria-label="Context categories"
                    class="flex gap-1 overflow-x-auto border-b border-border bg-surface p-2 md:block md:border-b-0 md:border-r"
                  >
                    <button
                      type="button"
                      role="tab"
                      aria-selected={activeContextSection === 'details'}
                      class={`flex min-w-32 items-center justify-between gap-3 rounded px-2.5 py-2 text-left text-sm transition-colors md:w-full ${activeContextSection === 'details' ? 'bg-bg text-fg' : 'text-fg-muted hover:bg-bg hover:text-fg'}`}
                      onclick={() => {
                        activeContextSection = 'details';
                      }}
                    >
                      <span>Prompt</span>
                      <span class="text-xs text-fg-subtle">
                        {taskRequirementTabLabel(selectedTask, 'details', taskRequirementState)}
                      </span>
                    </button>
                    <button
                      type="button"
                      role="tab"
                      aria-selected={activeContextSection === 'files'}
                      class={`flex min-w-32 items-center justify-between gap-3 rounded px-2.5 py-2 text-left text-sm transition-colors md:w-full ${activeContextSection === 'files' ? 'bg-bg text-fg' : 'text-fg-muted hover:bg-bg hover:text-fg'}`}
                      onclick={() => {
                        activeContextSection = 'files';
                      }}
                    >
                      <span>Files &amp; Directories</span>
                      <span class="text-xs text-fg-subtle">
                        {taskRequirementTabLabel(selectedTask, 'files', taskRequirementState)}
                      </span>
                    </button>
                    <button
                      type="button"
                      role="tab"
                      aria-selected={activeContextSection === 'records'}
                      class={`flex min-w-32 items-center justify-between gap-3 rounded px-2.5 py-2 text-left text-sm transition-colors md:w-full ${activeContextSection === 'records' ? 'bg-bg text-fg' : 'text-fg-muted hover:bg-bg hover:text-fg'}`}
                      onclick={() => {
                        activeContextSection = 'records';
                      }}
                    >
                      <span>Records</span>
                      <span class="text-xs text-fg-subtle">
                        {taskRequirementTabLabel(selectedTask, 'records', taskRequirementState)}
                      </span>
                    </button>
                  </div>

                  <div data-task-context-right-pane class="min-w-0 bg-bg p-3">
                    {#if activeContextSection === 'details'}
                      <section data-task-context-section="details" role="tabpanel" class="space-y-4">
                        {#if visibleTaskInputs(selectedTask).length}
                          <div class="grid gap-3 md:grid-cols-2">
                            {#each visibleTaskInputs(selectedTask) as input (input.name)}
                              <label
                                class={input.type === 'textarea' || input.type === 'select'
                                  ? 'block md:col-span-2'
                                  : 'block'}
                              >
                                {#if input.type === 'checkbox'}
                                  <span class="flex min-h-9 items-center gap-2 rounded-md border border-border bg-bg px-3 text-sm">
                                    <input
                                      type="checkbox"
                                      class="h-4 w-4 accent-accent"
                                      checked={Boolean(inputValues[input.name])}
                                      onchange={(event) => updateInput(input, event)}
                                    />
                                    <span>{normalizedInputLabel(input)}</span>
                                    {#if input.required}
                                      <span class="ml-auto text-[11px] text-fg-subtle">required</span>
                                    {/if}
                                  </span>
                                {:else if input.type === 'select'}
                                  <span class="flex min-h-9 flex-wrap items-center gap-3 text-sm">
                                    <span class="flex items-center gap-2 font-medium text-fg">
                                      <span>{normalizedInputLabel(input)}</span>
                                      {#if input.required}
                                        <span class="text-[11px] font-normal text-fg-subtle">required</span>
                                      {/if}
                                    </span>
                                    <select
                                      class="h-9 w-auto min-w-40 rounded-md border border-border bg-bg px-3 text-sm text-fg focus:outline-none focus:ring-2 focus:ring-accent"
                                      value={String(inputValues[input.name] ?? '')}
                                      required={input.required}
                                      onchange={(event) => updateInput(input, event)}
                                    >
                                      {#each input.options as option (option)}
                                        <option value={option}>{option}</option>
                                      {/each}
                                    </select>
                                  </span>
                                {:else}
                                  <span class="mb-1 flex items-center gap-2 text-sm font-medium text-fg">
                                    <span>{normalizedInputLabel(input)}</span>
                                    {#if input.required}
                                      <span class="text-[11px] font-normal text-fg-subtle">required</span>
                                    {/if}
                                  </span>
                                  {#if input.type === 'textarea'}
                                    <textarea
                                      class="min-h-36 w-full resize-y rounded-md border border-border bg-bg p-3 text-sm leading-6 text-fg focus:outline-none focus:ring-2 focus:ring-accent"
                                      value={String(inputValues[input.name] ?? '')}
                                      required={input.required}
                                      oninput={(event) => updateInput(input, event)}
                                    ></textarea>
                                  {:else if input.type === 'multiselect'}
                                    <select
                                      multiple
                                      class="min-h-28 w-full rounded-md border border-border bg-bg px-3 py-2 text-sm text-fg focus:outline-none focus:ring-2 focus:ring-accent"
                                      required={input.required}
                                      onchange={(event) => updateInput(input, event)}
                                    >
                                      {#each input.options as option (option)}
                                        <option value={option} selected={multiselectSelected(input, option)}>
                                          {option}
                                        </option>
                                      {/each}
                                    </select>
                                  {:else}
                                    <input
                                      type={input.type === 'number' ? 'number' : 'text'}
                                      class="h-9 w-full rounded-md border border-border bg-bg px-3 text-sm text-fg focus:outline-none focus:ring-2 focus:ring-accent"
                                      value={String(inputValues[input.name] ?? '')}
                                      required={input.required}
                                      oninput={(event) => updateInput(input, event)}
                                    />
                                  {/if}
                                {/if}
                              </label>
                            {/each}
                          </div>
                        {/if}
                        <label class="block">
                          <span class="mb-1 flex items-center gap-2 text-sm font-medium text-fg">
                            <span>Instructions and notes</span>
                            {#if detailsRequired(selectedTask)}
                              <span class="text-[11px] font-normal text-fg-subtle">required</span>
                            {/if}
                          </span>
                          <textarea
                            class="min-h-28 w-full resize-y rounded-md border border-border bg-bg p-3 text-sm leading-6 text-fg focus:outline-none focus:ring-2 focus:ring-accent"
                            bind:value={taskContext.instructions}
                            required={detailsRequired(selectedTask)}
                          ></textarea>
                        </label>
                      </section>
                    {:else if activeContextSection === 'files'}
                      <section data-task-context-section="files" role="tabpanel" class="space-y-5">
                        <TaskPathPicker
                          kind="file"
                          bind:value={filePathPickerQuery}
                          selectedPaths={selectedContextFilePaths}
                          onadd={addContextFilePath}
                          onremove={removeContextFilePath}
                        />
                        <TaskPathPicker
                          kind="directory"
                          bind:value={directoryPathPickerQuery}
                          selectedPaths={selectedContextDirectoryPaths}
                          onadd={addContextDirectoryPath}
                          onremove={removeContextDirectoryPath}
                        />
                      </section>
                    {:else if activeContextSection === 'records'}
                      <section data-task-context-section="records" role="tabpanel" class="space-y-3">
                        {#if scopeTaskInputs(selectedTask).length}
                          <div class="grid gap-3">
                            {#each scopeTaskInputs(selectedTask) as input (input.name)}
                              <label class="block">
                                <span class="flex min-h-9 flex-wrap items-center gap-3 text-sm">
                                  <span class="flex items-center gap-2 font-medium text-fg">
                                    <span>{normalizedInputLabel(input)}</span>
                                    {#if input.required}
                                      <span class="text-[11px] font-normal text-fg-subtle">required</span>
                                    {/if}
                                  </span>
                                  <select
                                    class="h-9 w-auto min-w-40 rounded-md border border-border bg-bg px-3 text-sm text-fg focus:outline-none focus:ring-2 focus:ring-accent"
                                    value={String(inputValues[input.name] ?? '')}
                                    required={input.required}
                                    onchange={(event) => updateInput(input, event)}
                                  >
                                    {#each input.options as option (option)}
                                      <option value={option}>{option}</option>
                                    {/each}
                                  </select>
                                </span>
                              </label>
                            {/each}
                          </div>
                        {/if}
                        {#if snapshot.isPending}
                          <p class="text-sm text-fg-muted">Loading records...</p>
                        {:else if snapshot.isError}
                          <div class="rounded-md border border-danger/40 bg-danger/5 p-3 text-sm text-danger">
                            {snapshot.error.message}
                          </div>
                        {:else}
                          <RecordPicker
                            {records}
                            selectedIds={selectedContextRecordIds}
                            bind:query={recordSearchQuery}
                            bind:status={recordStatusFilter}
                            onadd={addContextRecord}
                            onremove={removeContextRecord}
                          />
                        {/if}
                      </section>
                    {/if}
                  </div>
                </div>
              </section>

              {#if taskDiagnostics.length}
                <div class="space-y-2">
                  {#each taskDiagnostics as diagnostic (diagnostic.code + diagnostic.message)}
                    <div class="rounded-md border border-warning/40 bg-warning/5 p-3 text-sm">
                      <div class="font-medium text-warning">{diagnostic.code}</div>
                      <div class="mt-1 text-fg-muted">{diagnostic.message}</div>
                      {#if diagnostic.path || diagnostic.source_id}
                        <div class="mt-1 break-all font-mono text-xs text-fg-subtle">
                          {diagnostic.path ?? diagnostic.source_id}
                        </div>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {:else}
            <div class="rounded-md border border-dashed border-border p-6 text-sm text-fg-muted">
              No tasks available.
            </div>
          {/if}
      </TaskRunnerPane>
    {:else if selectedTab === 'runs'}
      <ResizableWorkbenchSplit
        splitId="task-runs"
        handleLabel="Resize task run panes"
        data-task-runs-workbench
        class="mt-4 p-0"
      >
        {#snippet left()}
        <WorkbenchPane framed data-task-runs-list-pane class="min-h-0 overflow-auto">
          <TaskHistoryPane
            {runs}
            {selectedRunId}
            {busy}
            loading={runsQuery.isPending}
            errorMessage={runsQuery.isError ? runsQuery.error.message : ''}
            refreshing={runsQuery.isFetching}
            clearing={clearTaskHistory.isPending}
            onRefresh={() => {
              void queryClient.invalidateQueries({ queryKey: ['task-runs'] });
            }}
            onClear={() => clearTaskHistory.mutate()}
            onLoadRun={selectTaskRun}
          />
        </WorkbenchPane>
        {/snippet}

        {#snippet right()}
        <WorkbenchPane framed data-task-runs-result-pane class="min-h-0 overflow-auto">
          <TaskResultsPanel
            runLabel={resultRunLabel}
            resultLabel={taskResult ? outputKindLabel(String(taskResult.kind)) : ''}
            status={selectedRun?.status ?? ''}
          >
        {#snippet actions()}
          {#if sourceRecordId}
              <a
                class="inline-flex h-8 items-center rounded-md border border-border bg-bg px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
                href={resolve(recordHref(sourceRecordId))}
              >
                Back to record
              </a>
              <button
                type="button"
                class="inline-flex h-8 items-center rounded-md border border-border bg-bg px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
                onclick={() => setTab('tasks')}
              >
                Run another task for this record
              </button>
          {/if}
        {/snippet}
        {#if taskResultLoading}
          <div class="rounded-md border border-dashed border-border p-6 text-sm text-fg-muted">
            Loading result...
          </div>
        {:else if selectedRun && isRunInProgress(selectedRun)}
          <div class="rounded-md border border-accent/30 bg-accent/5 p-4 text-sm text-fg-muted">
            <div class="font-medium text-fg">{inProgressRunMessage(selectedRun)}</div>
            <div class="mt-1">Results will appear here when the run finishes.</div>
          </div>
        {:else if selectedRun?.status === 'cancelled'}
          <div class="rounded-md border border-dashed border-border p-6 text-sm text-fg-muted">
            Task {selectedRun.task_id} was cancelled before producing a result.
          </div>
        {:else if taskResultError}
          <div class="rounded-md border border-danger/40 bg-danger/5 p-4 text-sm text-danger">
            {taskResultError}
          </div>
        {:else if !taskResult}
          <div class="rounded-md border border-dashed border-border p-6 text-sm text-fg-muted">
            Select a completed run.
          </div>
        {:else}
        {#if taskResult.kind === 'candidate_draft.v1'}
          <div class="space-y-4">
            <div>
              <h3 class="text-lg font-semibold">{stringField(taskResult, 'title') || 'Candidate draft'}</h3>
              <div class="mt-2 flex flex-wrap gap-1.5">
                {#each stringArrayField(taskResult, 'tags') as tag (tag)}
                  <span class="rounded bg-surface-2 px-1.5 py-0.5 text-[11px] text-fg-muted">{tag}</span>
                {/each}
              </div>
            </div>
            {#if stringArrayField(taskResult, 'warnings').length}
              <ul class="rounded-md border border-warning/40 bg-warning/5 p-3 text-sm text-warning">
                {#each stringArrayField(taskResult, 'warnings') as warning (warning)}
                  <li>{warning}</li>
                {/each}
              </ul>
            {/if}
            <textarea
              aria-label="Draft candidate markdown"
              bind:value={draftBody}
              class="min-h-72 w-full resize-y rounded-md border border-border bg-bg p-4 font-mono text-xs leading-6 text-fg focus:outline-none focus:ring-2 focus:ring-accent"
            ></textarea>
            <div class="flex justify-end">
              <button
                type="button"
                class="inline-flex h-8 items-center gap-1.5 rounded-md bg-accent px-3 text-sm font-medium text-accent-fg transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
                disabled={writeCandidate.isPending || !draftBody.trim()}
                onclick={writeDraftCandidate}
              >
                <FilePlus2 class="h-3.5 w-3.5" />
                <span>Write candidate</span>
              </button>
            </div>
          </div>
        {:else if taskResult.kind === 'candidate_batch.v1'}
          <div class="space-y-4">
            {#if summaryText(taskResult)}
              <details name="task-result-card" data-ui-stacked-card data-result-card class="rounded-md border border-border bg-bg">
                <summary class="cursor-pointer list-none px-4 py-3 text-sm font-medium text-fg [&::-webkit-details-marker]:hidden">
                  <span data-result-card-summary class="flex min-w-0 items-center gap-2">
                    <span class="text-fg-subtle group-open:rotate-90">▸</span>
                    <span class="truncate">Summary</span>
                  </span>
                </summary>
                <div class="record-markdown border-t border-border p-4">
                  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                  {@html renderMarkdown(summaryText(taskResult))}
                </div>
              </details>
            {/if}
            {#if candidateArray(taskResult).length}
              <div class="space-y-3">
                {#each candidateArray(taskResult) as candidate, index (candidate.title + index)}
                  <details name="task-result-card" data-ui-stacked-card data-result-card class="rounded-md border border-border bg-bg">
                    <summary class="cursor-pointer list-none px-4 py-3 [&::-webkit-details-marker]:hidden">
                      <div data-result-card-summary class="flex min-w-0 items-start gap-3">
                        <span class="mt-0.5 text-fg-subtle">▸</span>
                        <div class="min-w-0">
                          <h3 class="inline text-base font-semibold text-fg">{candidate.title}</h3>
                          <div class="mt-2 flex flex-wrap gap-1.5">
                            {#each candidate.tags as tag (tag)}
                              <span class="rounded bg-surface-2 px-1.5 py-0.5 text-[11px] text-fg-muted">{tag}</span>
                            {/each}
                          </div>
                        </div>
                      </div>
                    </summary>
                    <div class="border-t border-border p-4">
                      <div class="flex flex-wrap justify-end gap-2">
                      <button
                        type="button"
                        class="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-surface px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
                        onclick={() => startFollowUp(candidateFollowUp(candidate))}
                      >
                        <RefreshCw class="h-3.5 w-3.5" />
                        <span>Refine candidate</span>
                      </button>
                      </div>
                    {#if candidate.confidence}
                      <p class="mt-2 text-xs text-fg-subtle">Confidence: {candidate.confidence}</p>
                    {/if}
                    <div class="record-markdown mt-3">
                      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                      {@html renderMarkdown(candidate.body_markdown)}
                    </div>
                    </div>
                  </details>
                {/each}
              </div>
            {:else}
              <div class="rounded-md border border-dashed border-border p-4 text-sm text-fg-muted">
                No proposals returned.
              </div>
            {/if}
          </div>
        {:else if taskResult.kind === 'conflict_set.v1'}
          <div class="space-y-4">
            {#if stringField(taskResult, 'summary')}
              <details name="task-result-card" data-ui-stacked-card data-result-card class="rounded-md border border-border bg-bg">
                <summary class="cursor-pointer list-none px-4 py-3 text-sm font-medium text-fg [&::-webkit-details-marker]:hidden">
                  <span data-result-card-summary class="flex min-w-0 items-center gap-2">
                    <span class="text-fg-subtle">▸</span>
                    <span class="truncate">Summary</span>
                  </span>
                </summary>
                <div class="record-markdown border-t border-border p-4">
                  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                  {@html renderMarkdown(stringField(taskResult, 'summary'))}
                </div>
              </details>
            {/if}
            {#if objectArrayField(taskResult, 'findings').length}
              <div class="space-y-3">
                {#each objectArrayField(taskResult, 'findings') as finding, index (String(finding.id ?? index))}
                  {#if !isDismissed('conflict_set.v1', finding, index)}
                  <details name="task-result-card" data-ui-stacked-card data-result-card class="rounded-md border border-border bg-bg">
                    <summary class="cursor-pointer list-none px-4 py-3 [&::-webkit-details-marker]:hidden">
                      <div data-result-card-summary class="flex min-w-0 items-center gap-2">
                        <span class="text-fg-subtle">▸</span>
                        <h3 class="min-w-0 truncate text-base font-semibold text-fg">{String(finding.title ?? finding.id ?? 'Conflict')}</h3>
                      </div>
                    </summary>
                    <div class="border-t border-border p-4">
                      <div class="flex flex-wrap gap-2 text-xs">
                        {#if typeof finding.left_record_id === 'string'}
                          <a class="font-mono text-accent hover:underline" href={resolve(`/records?id=${finding.left_record_id}`)}>
                            {finding.left_record_id}
                          </a>
                        {/if}
                        {#if typeof finding.right_record_id === 'string'}
                          <a class="font-mono text-accent hover:underline" href={resolve(`/records?id=${finding.right_record_id}`)}>
                            {finding.right_record_id}
                          </a>
                        {/if}
                      </div>
                      <p class="mt-3 text-sm text-fg-muted">{String(finding.conflict_explanation ?? '')}</p>
                      <div class="mt-3 flex flex-wrap gap-2">
                        <button
                          type="button"
                          class="inline-flex h-8 items-center rounded-md border border-border bg-surface px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
                          onclick={() => dismissResultItem('conflict_set.v1', finding, index)}
                        >
                          Dismiss
                        </button>
                      </div>
                    </div>
                  </details>
                  {/if}
                {/each}
              </div>
            {:else}
              <div class="rounded-md border border-dashed border-border p-4 text-sm text-fg-muted">
                No conflicts returned.
              </div>
            {/if}
          </div>
        {:else if taskResult.kind === 'diagnosis_report.v1'}
          <div class="space-y-4">
            {#if summaryText(taskResult)}
              <details name="task-result-card" data-ui-stacked-card data-result-card class="rounded-md border border-border bg-bg">
                <summary class="cursor-pointer list-none px-4 py-3 text-sm font-medium text-fg [&::-webkit-details-marker]:hidden">
                  <span data-result-card-summary class="flex min-w-0 items-center gap-2">
                    <span class="text-fg-subtle">▸</span>
                    <span class="truncate">Summary</span>
                  </span>
                </summary>
                <div class="record-markdown border-t border-border p-4">
                  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                  {@html renderMarkdown(summaryText(taskResult))}
                </div>
              </details>
            {/if}
            {#each objectArrayField(taskResult, 'issues') as issue, index (String(issue.id ?? index))}
              {#if !isDismissed('diagnosis_report.v1', issue, index)}
              <details name="task-result-card" data-ui-stacked-card data-result-card class="rounded-md border border-border bg-bg">
                <summary class="cursor-pointer list-none px-4 py-3 [&::-webkit-details-marker]:hidden">
                  <div data-result-card-summary class="flex min-w-0 flex-wrap items-center gap-2">
                    <span class="shrink-0 text-fg-subtle">▸</span>
                    <h3 class="min-w-0 flex-1 text-base font-semibold text-fg">{String(issue.title ?? 'Issue')}</h3>
                    <span class="rounded bg-surface-2 px-1.5 py-0.5 text-[11px] text-fg-muted">
                      {String(issue.severity ?? 'info')}
                    </span>
                    <span class="rounded bg-surface-2 px-1.5 py-0.5 text-[11px] text-fg-muted">
                      {String(issue.category ?? 'issue')}
                    </span>
                  </div>
                </summary>
                <div class="border-t border-border p-4">
                  <p class="text-sm text-fg-muted">{String(issue.explanation ?? '')}</p>
                  <div class="mt-3 flex flex-wrap gap-2">
                    {#if typeof issue.record_id === 'string' && issue.record_id}
                      <a
                        class="inline-flex h-8 items-center rounded-md border border-border bg-surface px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
                        href={resolve(`/records?id=${issue.record_id}`)}
                      >
                        Open record
                      </a>
                    {/if}
                    <button
                      type="button"
                      class="inline-flex h-8 items-center rounded-md border border-border bg-surface px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
                      onclick={() => dismissResultItem('diagnosis_report.v1', issue, index)}
                    >
                      Dismiss
                    </button>
                  </div>
                </div>
              </details>
              {/if}
            {/each}
          </div>
        {:else if taskResult.kind === 'record_revision.v1'}
          {@const revisionRecordId = stringField(taskResult, 'record_id')}
          <div class="space-y-4">
            <div class="rounded-md border border-border bg-bg p-4">
              <div class="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <h3 class="text-lg font-semibold">Review revision</h3>
                  {#if stringField(taskResult, 'summary')}
                    <h4 class="mt-2 text-base font-semibold text-fg">{stringField(taskResult, 'summary')}</h4>
                  {/if}
                </div>
                {#if revisionRecordId}
                  <a
                    class="inline-flex h-8 items-center rounded-md border border-border bg-surface px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
                    href={resolve(`/records?id=${revisionRecordId}`)}
                  >
                    Open record
                  </a>
                  <button
                    type="button"
                    class="inline-flex h-8 items-center rounded-md border border-border bg-surface px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
                    onclick={() => startTaskForRecord(revisionRecordId)}
                  >
                    Run another task for this record
                  </button>
                {/if}
              </div>
              {#if stringField(taskResult, 'change_explanation')}
                <div class="mt-3 rounded-md border border-border bg-surface p-3 text-sm text-fg-muted">
                  {stringField(taskResult, 'change_explanation')}
                </div>
              {/if}
              {#if stringArrayField(taskResult, 'warnings').length}
                <ul class="mt-3 rounded-md border border-warning/40 bg-warning/5 p-3 text-sm text-warning">
                  {#each stringArrayField(taskResult, 'warnings') as warning (warning)}
                    <li>{warning}</li>
                  {/each}
                </ul>
              {/if}
            </div>
            <div class="grid gap-3 md:grid-cols-2">
              <label class="block">
                <span class="mb-1 block text-sm font-medium text-fg">Revision title</span>
                <input
                  type="text"
                  class="h-9 w-full rounded-md border border-border bg-bg px-3 text-sm text-fg focus:outline-none focus:ring-2 focus:ring-accent"
                  bind:value={revisionTitle}
                />
              </label>
              <label class="block">
                <span class="mb-1 block text-sm font-medium text-fg">Revision tags</span>
                <textarea
                  aria-label="Revision tags"
                  class="min-h-9 w-full resize-y rounded-md border border-border bg-bg px-3 py-2 font-mono text-xs leading-5 text-fg focus:outline-none focus:ring-2 focus:ring-accent"
                  bind:value={revisionTags}
                ></textarea>
              </label>
            </div>
            <label class="block">
              <span class="mb-2 block text-sm font-medium text-fg">Revised record markdown</span>
              <textarea
                aria-label="Revised record markdown"
                class="min-h-72 w-full resize-y rounded-md border border-border bg-bg p-4 font-mono text-xs leading-6 text-fg focus:outline-none focus:ring-2 focus:ring-accent"
                bind:value={revisionBody}
              ></textarea>
            </label>
            <div class="flex flex-wrap justify-end gap-2">
              <button
                type="button"
                class="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-surface px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
                onclick={startRecordRevisionFollowUp}
              >
                <RefreshCw class="h-3.5 w-3.5" />
                <span>Refine with agent</span>
              </button>
              <button
                type="button"
                class="inline-flex h-8 items-center gap-1.5 rounded-md bg-accent px-3 text-sm font-medium text-accent-fg transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
                disabled={applyRecordRevision.isPending || !revisionTitle.trim() || !revisionBody.trim()}
                onclick={() => applyRecordRevision.mutate()}
              >
                <Check class="h-3.5 w-3.5" />
                <span>{applyRecordRevision.isPending ? 'Applying' : 'Apply revision'}</span>
              </button>
            </div>
          </div>
        {:else if taskResult.kind === 'supersession_proposal.v1'}
          {@const supersededRecordId = stringField(taskResult, 'superseded_record_id')}
          <div class="space-y-4">
            <div class="rounded-md border border-border bg-bg p-4">
              <div class="flex flex-wrap items-start justify-between gap-3">
                <h3 class="text-lg font-semibold">{stringField(taskResult, 'superseding_title') || 'Supersession proposal'}</h3>
                {#if supersededRecordId}
                  <div class="flex flex-wrap gap-2">
                    <a
                      class="inline-flex h-8 items-center rounded-md border border-border bg-surface px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
                      href={resolve(`/records?id=${supersededRecordId}`)}
                    >
                      Open record
                    </a>
                    <button
                      type="button"
                      class="inline-flex h-8 items-center rounded-md border border-border bg-surface px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
                      onclick={() => startTaskForRecord(supersededRecordId)}
                    >
                      Run another task for this record
                    </button>
                  </div>
                {/if}
              </div>
              <p class="mt-2 text-sm text-fg-muted">{stringField(taskResult, 'summary')}</p>
              <p class="mt-3 text-sm text-fg-muted">{stringField(taskResult, 'rationale')}</p>
              <div class="record-markdown mt-4">
                <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                {@html renderMarkdown(stringField(taskResult, 'superseding_body_markdown'))}
              </div>
            </div>
            <div class="flex justify-end">
              <button
                type="button"
                class="inline-flex h-8 items-center gap-1.5 rounded-md bg-accent px-3 text-sm font-medium text-accent-fg transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
                disabled={writeCandidate.isPending || !stringField(taskResult, 'superseding_body_markdown')}
                onclick={writeSupersessionCandidate}
              >
                <FilePlus2 class="h-3.5 w-3.5" />
                <span>Write candidate</span>
              </button>
            </div>
          </div>
        {:else}
          <div class="space-y-4">
            {#if stringField(taskResult, 'title') || stringField(taskResult, 'summary')}
              <details name="task-result-card" data-ui-stacked-card data-result-card class="rounded-md border border-border bg-bg">
                <summary class="cursor-pointer list-none px-4 py-3 text-base font-semibold text-fg [&::-webkit-details-marker]:hidden">
                  <span data-result-card-summary class="flex min-w-0 items-center gap-2">
                    <span class="text-fg-subtle">▸</span>
                    <span class="truncate">{stringField(taskResult, 'title') || stringField(taskResult, 'summary')}</span>
                  </span>
                </summary>
                {#if stringField(taskResult, 'body_markdown')}
                  <div class="record-markdown border-t border-border p-4">
                    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                    {@html renderMarkdown(stringField(taskResult, 'body_markdown'))}
                  </div>
                {/if}
              </details>
            {:else if stringField(taskResult, 'body_markdown')}
              <details name="task-result-card" data-ui-stacked-card data-result-card class="rounded-md border border-border bg-bg">
                <summary class="cursor-pointer list-none px-4 py-3 text-base font-semibold text-fg [&::-webkit-details-marker]:hidden">
                  <span data-result-card-summary class="flex min-w-0 items-center gap-2">
                    <span class="text-fg-subtle">▸</span>
                    <span class="truncate">Result</span>
                  </span>
                </summary>
                <div class="record-markdown border-t border-border p-4">
                  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                  {@html renderMarkdown(stringField(taskResult, 'body_markdown'))}
                </div>
              </details>
            {:else}
              <details name="task-result-card" data-ui-stacked-card data-result-card class="rounded-md border border-border bg-bg">
                <summary class="cursor-pointer list-none px-4 py-3 text-base font-semibold text-fg [&::-webkit-details-marker]:hidden">
                  <span data-result-card-summary class="flex min-w-0 items-center gap-2">
                    <span class="text-fg-subtle">▸</span>
                    <span class="truncate">Result</span>
                  </span>
                </summary>
                <div class="border-t border-border p-4 text-sm text-fg-muted">
                  This result type is not yet displayable.
                </div>
              </details>
            {/if}
          </div>
        {/if}
        {/if}
        {#if followUpTarget}
          <TaskFollowUpComposer
            title={followUpTarget.title}
            bind:instructions={followUpInstructions}
            running={runFollowUp.isPending}
            onCancel={() => {
              followUpTarget = null;
              followUpInstructions = '';
            }}
            onRun={() => runFollowUp.mutate()}
          />
        {/if}
          </TaskResultsPanel>
        </WorkbenchPane>
        {/snippet}
      </ResizableWorkbenchSplit>
    {/if}
  </div>
    </div>
  </WorkbenchPane>
</WorkbenchPage>
