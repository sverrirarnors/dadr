<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { createApi } from '$lib/api';
  import {
    buildCommandPaletteItems,
    filterCommandPaletteItems,
    type CommandPaletteItem,
    type CommandPaletteRoute,
  } from '$lib/command-palette/items';
  import { ledgerUrl } from '$lib/navigation/routes';
  import { palette } from '$lib/stores/palette.svelte';
  import { theme } from '$lib/stores/theme.svelte';
  import { toasts } from '$lib/stores/toast.svelte';

  const api = createApi();
  const queryClient = useQueryClient();

  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  let query = $state('');
  let inputEl = $state<HTMLInputElement | undefined>();

  $effect(() => {
    if (palette.open && inputEl) {
      queueMicrotask(() => inputEl?.focus());
    }
    if (!palette.open) {
      query = '';
    }
  });

  function handleKey(event: KeyboardEvent) {
    const isK = event.key.toLowerCase() === 'k' || event.code === 'KeyK';
    if ((event.metaKey || event.ctrlKey) && isK) {
      event.preventDefault();
      palette.toggle();
    }
    if (event.key === 'Escape' && palette.open) {
      palette.hide();
    }
  }

  const items = $derived(buildCommandPaletteItems(snapshot.data?.records ?? []));
  const filtered = $derived(filterCommandPaletteItems(items, query));

  async function pick(item: CommandPaletteItem) {
    palette.hide();
    if (item.recordId) {
      const record = snapshot.data?.records.find((candidate) => candidate.id === item.recordId);
      const history = record ? record.status !== 'accepted' || record.superseded : false;
      goto(resolve(ledgerUrl({ id: item.recordId, history })));
      return;
    }
    if (item.route) {
      goRoute(item.route);
      return;
    }

    switch (item.action) {
      case 'toggle-theme':
        theme.cycle();
        toasts.push({ severity: 'info', message: `Theme set to ${theme.current}` });
        break;
      case 'reload-snapshot':
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ['snapshot'] }),
          queryClient.invalidateQueries({ queryKey: ['inbox-runs'] }),
        ]);
        toasts.push({ severity: 'info', message: 'Reloaded' });
        break;
    }
  }

  function goRoute(route: CommandPaletteRoute) {
    goto(resolve(route));
  }
</script>

<svelte:window onkeydown={handleKey} />

{#if palette.open}
  <div
    class="fixed inset-0 z-50 flex items-start justify-center bg-fg/30 pt-32 backdrop-blur-sm"
    role="presentation"
    onclick={() => palette.hide()}
  >
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      tabindex="-1"
      class="w-full max-w-md overflow-hidden rounded-xl border border-border bg-bg shadow-2xl"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.stopPropagation()}
    >
      <input
        bind:this={inputEl}
        bind:value={query}
        type="text"
        placeholder="Search commands..."
        class="h-12 w-full border-b border-border bg-bg px-4 text-sm text-fg placeholder:text-fg-subtle focus:outline-none"
      />
      <ul class="max-h-80 overflow-y-auto py-1">
        {#each filtered as item (item.id)}
          <li>
            <button
              type="button"
              class="grid w-full grid-cols-[5.5rem_1fr] gap-3 px-4 py-2 text-left text-sm text-fg transition-colors hover:bg-surface-2"
              onclick={() => pick(item)}
            >
              <span class="pt-0.5 text-[11px] font-medium uppercase text-fg-subtle">{item.section}</span>
              <span class="min-w-0">
                <span class="block truncate">{item.label}</span>
                {#if item.detail}
                  <span class="mt-0.5 block truncate text-xs text-fg-subtle">{item.detail}</span>
                {/if}
              </span>
            </button>
          </li>
        {/each}
        {#if filtered.length === 0}
          <li class="px-4 py-3 text-sm text-fg-subtle">No matches.</li>
        {/if}
      </ul>
    </div>
  </div>
{/if}
