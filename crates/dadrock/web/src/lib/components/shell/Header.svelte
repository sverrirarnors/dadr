<script lang="ts">
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { createQuery } from '@tanstack/svelte-query';
  import { PenLine } from 'lucide-svelte';
  import { createApi } from '$lib/api';
  import { isPrimaryNavItemActive, primaryNavItems } from '$lib/navigation/routes';
  import { composer } from '$lib/stores/composer.svelte';
  import ProjectGear from './ProjectGear.svelte';
  import ThemeToggle from './ThemeToggle.svelte';

  const api = createApi();
  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  const candidateCount = $derived(snapshot.data?.counts.candidates ?? 0);
  const initialized = $derived(snapshot.data?.project.initialized ?? false);
</script>

<header data-app-header class="flex h-11 items-center gap-3 border-b border-border bg-bg px-3">
  <ProjectGear />

  <nav class="flex items-center gap-1" aria-label="Primary navigation">
    {#each primaryNavItems as item (item.href)}
      <a
        href={resolve(item.href)}
        aria-label={item.ariaLabel}
        aria-current={isPrimaryNavItemActive(item, page.url) ? 'page' : undefined}
        class="inline-flex h-8 items-center gap-1.5 rounded-md px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        class:bg-surface-2={isPrimaryNavItemActive(item, page.url)}
        class:text-fg={isPrimaryNavItemActive(item, page.url)}
      >
        <span>{item.label}</span>
        {#if item.id === 'inbox' && candidateCount > 0}
          <span
            data-inbox-count
            class="inline-flex min-w-5 items-center justify-center rounded-full bg-accent px-1.5 text-[11px] font-medium leading-5 text-accent-fg"
            aria-label={`${candidateCount} candidates waiting`}
          >
            {candidateCount}
          </span>
        {/if}
      </a>
    {/each}
  </nav>

  <div class="flex-1"></div>

  <button
    type="button"
    data-draft-button
    class="inline-flex h-8 items-center gap-1.5 rounded-md bg-accent px-3 text-sm font-medium text-accent-fg transition-opacity hover:opacity-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-50"
    disabled={!initialized}
    title={initialized ? 'Draft a decision' : 'Initialize the project first'}
    onclick={() => composer.show({ mode: 'new' })}
  >
    <PenLine class="h-3.5 w-3.5" aria-hidden="true" />
    <span>Draft</span>
  </button>

  <ThemeToggle />
</header>
