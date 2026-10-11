<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import { Send, X } from 'lucide-svelte';
  import { createApi } from '$lib/api';
  import RecordSelectionDialog from '$lib/components/records/RecordSelectionDialog.svelte';
  import { Alert, Button, TextareaInput } from '$lib/components/ui';
  import {
    attachRecordsStatus,
    composerTitle,
    errorMessage,
    initialComposerView,
    ledgerEditHref,
    manualFormAvailable,
    pinnedRecord,
    promptLabel,
    type ComposerView,
  } from '$lib/draft/composer-view';
  import { resolveDraftIntent } from '$lib/draft/intent';
  import { inboxUrl, ledgerUrl } from '$lib/navigation/routes';
  import { applyMutationResponse } from '$lib/query/mutation-cache';
  import type { RecordStatusFilter } from '$lib/records/filters';
  import { composer } from '$lib/stores/composer.svelte';
  import { toasts } from '$lib/stores/toast.svelte';
  import DraftAttachments from './DraftAttachments.svelte';
  import DraftManualForm from './DraftManualForm.svelte';

  const api = createApi();
  const queryClient = useQueryClient();
  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  const loaded = $derived(Boolean(snapshot.data));
  const records = $derived(snapshot.data?.records ?? []);
  const agentAvailable = $derived(snapshot.data?.settings.agent_available ?? false);
  const pinned = $derived(pinnedRecord(composer.mode, composer.recordIds, records));
  const title = $derived(composerTitle(composer.mode, pinned));
  const intent = $derived(
    resolveDraftIntent(
      {
        mode: composer.mode,
        text: composer.text,
        paths: composer.paths,
        recordIds: composer.recordIds,
      },
      records,
    ),
  );

  /** The person's explicit choice; null follows the default for the mode. */
  let chosenView = $state<ComposerView | null>(null);
  const view = $derived(chosenView ?? initialComposerView(composer.mode, agentAvailable));
  const canWriteByHand = $derived(manualFormAvailable(composer.mode));
  const canAttachRecords = $derived(
    composer.mode === 'new' || composer.mode === 'merge' || !pinned,
  );
  /** The record "Edit it in the Ledger" points at, in revise mode. */
  const editRecord = $derived(composer.mode === 'revise' ? pinned : undefined);

  let sending = $state(false);
  let error = $state<{ title: string; detail: string } | null>(null);
  let picker = $state<'attach' | 'replace' | null>(null);
  let pickerQuery = $state('');
  let pickerStatus = $state<RecordStatusFilter>('all');
  let sheet = $state<HTMLElement | undefined>();

  let focusedForOpen = false;

  $effect(() => {
    if (!composer.open) {
      focusedForOpen = false;
      chosenView = null;
      sending = false;
      error = null;
      picker = null;
      pickerQuery = '';
      return;
    }
    if (focusedForOpen || !loaded) return;
    focusedForOpen = true;
    void tick().then(() => {
      sheet?.querySelector<HTMLElement>('[data-autofocus]')?.focus();
    });
  });

  /** Escape with focus inside the sheet. */
  function handleSheetKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    event.stopPropagation();
    composer.hide();
  }

  /** Escape with focus anywhere else, except in the record picker above the sheet. */
  function handleWindowKeydown(event: KeyboardEvent) {
    if (!composer.open || event.key !== 'Escape') return;
    if (picker) return;
    const target = event.target as Element | null;
    if (target?.closest('[data-record-picker-dialog]')) return;
    event.preventDefault();
    composer.hide();
  }

  function openAttachPicker() {
    pickerQuery = '';
    pickerStatus = attachRecordsStatus(composer.mode);
    picker = 'attach';
  }

  function openReplacePicker() {
    pickerQuery = '';
    pickerStatus = 'accepted';
    picker = 'replace';
  }

  function attachRecords(ids: string[]) {
    picker = null;
    const next = [...composer.recordIds];
    for (const id of ids) {
      if (!next.includes(id)) next.push(id);
    }
    composer.recordIds = next;
  }

  async function send() {
    if (!intent.ok || sending) return;
    const request = intent.intent.request;
    sending = true;
    error = null;
    try {
      const run = await api.runTask(request);
      if (run.status === 'failed') {
        error = {
          title: 'The agent could not start',
          detail: `Run ${run.id} failed before it produced anything. Check the agent log in the Inbox.`,
        };
        return;
      }
      toasts.push({ severity: 'success', message: 'Sent to the agent. Results will appear in the Inbox.' });
      void queryClient.invalidateQueries({ queryKey: ['inbox-runs'] });
      composer.reset();
      await goto(resolve(inboxUrl() as '/inbox'));
    } catch (cause) {
      error = { title: 'The agent could not start', detail: errorMessage(cause) };
    } finally {
      sending = false;
    }
  }

  async function replaceWithExisting(ids: string[]) {
    picker = null;
    const from = pinned?.id;
    if (!from || ids.length === 0 || sending) return;
    sending = true;
    error = null;
    try {
      const response = await api.supersedeRecords({ from, to: ids });
      toasts.push({ severity: 'success', message: response.message });
      await applyMutationResponse(queryClient, response);
      composer.reset();
      await goto(resolve(ledgerUrl({ id: from, history: true }) as '/ledger'));
    } catch (cause) {
      error = { title: 'Could not replace the decision', detail: errorMessage(cause) };
    } finally {
      sending = false;
    }
  }

  const linkClass =
    'text-sm text-fg-muted underline-offset-2 hover:text-fg hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent';
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if composer.open}
  <div
    data-draft-composer
    class="fixed inset-0 z-40 flex justify-end bg-fg/30"
    role="presentation"
    onclick={() => composer.hide()}
  >
    <div
      bind:this={sheet}
      role="dialog"
      aria-modal="true"
      aria-label="Draft"
      tabindex="-1"
      class="flex h-full w-full max-w-full flex-col border-l border-border bg-bg shadow-2xl focus:outline-none md:max-w-xl"
      in:fly={{ x: 24, duration: 160 }}
      onclick={(event) => event.stopPropagation()}
      onkeydown={handleSheetKeydown}
    >
      <header class="flex h-11 shrink-0 items-center gap-3 border-b border-border px-4">
        <h2 class="min-w-0 flex-1 truncate text-sm font-semibold text-fg">{title}</h2>
        <button
          type="button"
          class="-mr-1.5 inline-flex h-8 w-8 items-center justify-center rounded-md text-fg-muted hover:bg-surface-2 hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
          aria-label="Close"
          onclick={() => composer.hide()}
        >
          <X class="h-4 w-4" aria-hidden="true" />
        </button>
      </header>

      <div class="min-h-0 flex-1 overflow-y-auto px-4 py-4">
        {#if !loaded}
          <p class="text-sm text-fg-subtle">Loading the project…</p>
        {:else if view === 'manual'}
          <DraftManualForm
            mode={composer.mode}
            record={pinned}
            {agentAvailable}
            onuseagent={() => (chosenView = 'agent')}
          />
        {:else if !agentAvailable}
          <div class="space-y-3">
            <Alert variant="warning" title="No local agent found">
              Install a provider such as claude-agent-acp or set DADROCK_ACP_AGENT, then reopen Draft.
            </Alert>
            {#if editRecord}
              <a
                href={resolve(ledgerEditHref(editRecord) as '/ledger')}
                class={linkClass}
                onclick={() => composer.hide()}
              >
                Edit it in the Ledger
              </a>
            {/if}
          </div>
        {:else}
          <div class="space-y-4">
            <label class="block space-y-1.5">
              <span class="block text-sm text-fg">{promptLabel(composer.mode)}</span>
              <TextareaInput
                data-autofocus
                data-draft-prompt
                aria-label={promptLabel(composer.mode)}
                class="min-h-32"
                bind:value={composer.text}
                disabled={sending}
              />
            </label>

            <DraftAttachments
              {records}
              pinnedId={pinned?.id}
              {canAttachRecords}
              onattachrecords={openAttachPicker}
            />

            <p data-draft-intent class="text-xs text-fg-muted" aria-live="polite">
              {intent.ok ? intent.intent.label : intent.reason}
            </p>

            {#if error}
              <Alert variant="danger" title={error.title}>{error.detail}</Alert>
            {/if}
          </div>
        {/if}

        {#if loaded && error && view === 'manual'}
          <Alert class="mt-4" variant="danger" title={error.title}>{error.detail}</Alert>
        {/if}

        {#if loaded && composer.mode === 'supersede' && pinned}
          <div class="mt-6 border-t border-border pt-4">
            <p class="text-xs text-fg-subtle">Already decided elsewhere?</p>
            <Button
              type="button"
              variant="outline"
              size="sm"
              class="mt-2"
              disabled={sending}
              onclick={openReplacePicker}
            >
              Replace with existing decisions instead
            </Button>
          </div>
        {/if}
      </div>

      {#if loaded && view === 'agent' && agentAvailable}
        <footer class="flex shrink-0 flex-wrap items-center gap-3 border-t border-border px-4 py-3">
          <Button type="button" disabled={!intent.ok || sending} onclick={send}>
            <Send class="h-3.5 w-3.5" aria-hidden="true" />
            <span>{sending ? 'Sending' : 'Send to agent'}</span>
          </Button>
          {#if canWriteByHand}
            <button type="button" class={linkClass} onclick={() => (chosenView = 'manual')}>Write it by hand</button>
          {:else if editRecord}
            <a
              href={resolve(ledgerEditHref(editRecord) as '/ledger')}
              class={linkClass}
              onclick={() => composer.hide()}
            >
              Edit it in the Ledger
            </a>
          {/if}
        </footer>
      {/if}
    </div>
  </div>

  {#if picker === 'attach'}
    <RecordSelectionDialog
      title="Attach records"
      {records}
      excludedIds={composer.recordIds}
      bind:query={pickerQuery}
      bind:status={pickerStatus}
      confirmLabel={(count) => (count === 1 ? 'Attach 1 record' : `Attach ${count} records`)}
      onconfirm={attachRecords}
      oncancel={() => (picker = null)}
    />
  {:else if picker === 'replace' && pinned}
    <RecordSelectionDialog
      title="Replace with existing decisions"
      {records}
      excludedIds={[pinned.id]}
      bind:query={pickerQuery}
      bind:status={pickerStatus}
      confirmLabel={() => 'Save'}
      onconfirm={replaceWithExisting}
      oncancel={() => (picker = null)}
    />
  {/if}
{/if}
