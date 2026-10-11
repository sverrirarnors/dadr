<script lang="ts">
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { createQuery } from '@tanstack/svelte-query';
  import { BookOpen, Inbox, PenLine } from 'lucide-svelte';
  import { createApi } from '$lib/api';
  import { isPrimaryNavItemActive, primaryNavItems, type PrimaryNavId } from '$lib/navigation/routes';
  import ProjectGear from './ProjectGear.svelte';
  import ThemeToggle from './ThemeToggle.svelte';

  const api = createApi();
  const snapshot = createQuery(() => ({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  }));

  const icons = {
    inbox: Inbox,
    ledger: BookOpen,
    draft: PenLine,
  } satisfies Record<PrimaryNavId, typeof Inbox>;

  const candidateCount = $derived(snapshot.data?.counts.candidates ?? 0);
  const initialized = $derived(snapshot.data?.project.initialized ?? false);
</script>

<header data-app-header class="flex h-11 items-center gap-3 border-b border-border bg-bg px-3">
  <ProjectGear />

  <nav class="flex items-center gap-1" aria-label="Primary navigation">
    {#each primaryNavItems as item (item.href)}
      {@const Icon = icons[item.id]}
      {@const active = isPrimaryNavItemActive(item, page.url)}
      {@const disabled = item.id === 'draft' && !initialized}
      <a
        href={resolve(item.href)}
        aria-label={item.ariaLabel}
        aria-current={active ? 'page' : undefined}
        aria-disabled={disabled ? 'true' : undefined}
        data-draft-button={item.id === 'draft' ? '' : undefined}
        title={disabled ? 'Initialize the project first' : item.label}
        class="inline-flex h-8 items-center gap-1.5 rounded-md px-2.5 text-sm text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        class:bg-surface-2={active}
        class:text-fg={active}
        class:pointer-events-none={disabled}
        class:opacity-50={disabled}
        tabindex={disabled ? -1 : undefined}
      >
        <Icon class="h-4 w-4" aria-hidden="true" />
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

  <ThemeToggle />
</header>
