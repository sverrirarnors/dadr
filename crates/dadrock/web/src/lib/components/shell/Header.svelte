<script lang="ts">
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import {
    FileText,
    Hammer,
    Inbox,
    Settings,
    ShieldCheck,
    Tag,
  } from 'lucide-svelte';
  import {
    isPrimaryNavItemActive,
    primaryNavItems,
    type PrimaryNavId,
  } from '$lib/navigation/routes';
  import ThemeToggle from './ThemeToggle.svelte';

  const icons = {
    inbox: Inbox,
    records: FileText,
    tags: Tag,
    tasks: Hammer,
    validation: ShieldCheck,
    settings: Settings,
  } satisfies Record<PrimaryNavId, typeof Inbox>;

  const navItems = primaryNavItems.map((item) => ({ ...item, icon: icons[item.id] }));
</script>

<header data-app-header class="flex h-11 items-center gap-2 border-b border-border bg-bg px-2">
  <div class="text-sm font-medium text-fg">dadrock</div>

  <nav class="flex items-center gap-1" aria-label="Primary navigation">
    {#each navItems as item (item.href)}
      <a
        href={resolve(item.href)}
        aria-label={item.ariaLabel}
        title={item.label}
        class="group relative inline-flex h-8 w-8 items-center justify-center rounded-md text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
        class:bg-surface-2={isPrimaryNavItemActive(item, page.url)}
        class:!text-fg={isPrimaryNavItemActive(item, page.url)}
      >
        <item.icon class="h-4 w-4" />
        <span
          class="pointer-events-none absolute left-1/2 top-full z-50 mt-1 hidden -translate-x-1/2 whitespace-nowrap rounded-md border border-border bg-bg px-2 py-1 text-xs font-medium text-fg shadow-lg group-hover:block group-focus-visible:block"
        >
          {item.label}
        </span>
      </a>
    {/each}
  </nav>

  <div class="flex-1"></div>

  <ThemeToggle />
</header>
