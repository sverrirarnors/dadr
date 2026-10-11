<script lang="ts">
  import '../app.css';
  import { QueryClientProvider } from '@tanstack/svelte-query';
  import Header from '$lib/components/shell/Header.svelte';
  import ToastRegion from '$lib/components/shell/ToastRegion.svelte';
  import CommandPalette from '$lib/components/shell/CommandPalette.svelte';
  import { createQueryClient } from '$lib/query';
  import { openComposerFromUrl } from '$lib/navigation/deep-links';
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();
  const queryClient = createQueryClient();
  const workbenchSurface = $derived(
    ['/inbox', '/ledger'].some((prefix) => page.url.pathname.startsWith(prefix)),
  );

  $effect(() => {
    openComposerFromUrl(page.url);
  });

  onMount(() => {
    const html = document.documentElement;
    const vscodeHost = page.url.searchParams.get('vscode') === '1' || html.dataset.host === 'vscode';
    if (!vscodeHost) return;

    html.dataset.host = 'vscode';
    const onMessage = (event: MessageEvent) => {
      const data = event.data;
      if (data?.kind !== 'vscode-theme') return;
      if (data.theme === 'light' || data.theme === 'dark' || data.theme === 'hc') {
        html.dataset.theme = data.theme;
      }
      if (data.vars && typeof data.vars === 'object') {
        for (const [key, value] of Object.entries(data.vars)) {
          if (key.startsWith('--vscode-') && typeof value === 'string') {
            html.style.setProperty(key, value);
          }
        }
      }
    };

    window.addEventListener('message', onMessage);
    window.parent.postMessage({ kind: 'ready' }, '*');
    return () => window.removeEventListener('message', onMessage);
  });
</script>

<QueryClientProvider client={queryClient}>
  <div data-app-shell class="flex h-screen flex-col bg-bg text-fg">
    <Header />
    <main class="min-h-0 flex-1" class:overflow-hidden={workbenchSurface} class:overflow-auto={!workbenchSurface}>
      {@render children()}
    </main>
  </div>
  <ToastRegion />
  <CommandPalette />
</QueryClientProvider>
