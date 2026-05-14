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
  import ThemeToggle from './ThemeToggle.svelte';

  const navItems = [
    { href: '/records?status=candidate', label: 'Inbox', ariaLabel: 'Candidate inbox', icon: Inbox },
    { href: '/records', label: 'Records', ariaLabel: 'Browse records', icon: FileText },
    { href: '/tags', label: 'Tags', ariaLabel: 'Browse tag list', icon: Tag },
    { href: '/tasks', label: 'Tasks', ariaLabel: 'Task runner', icon: Hammer },
    { href: '/validation', label: 'Validation', ariaLabel: 'Project validation', icon: ShieldCheck },
    { href: '/settings', label: 'Settings', ariaLabel: 'Project settings', icon: Settings },
  ] as const;

  type NavHref = (typeof navItems)[number]['href'];

  function routeHref(href: NavHref) {
    return resolve(href);
  }

  function isActive(href: NavHref) {
    if (href === '/records?status=candidate') {
      return page.url.pathname === '/records' && page.url.searchParams.get('status') === 'candidate';
    }
    if (href === '/records') {
      return page.url.pathname === '/records' && page.url.searchParams.get('status') !== 'candidate';
    }
    return page.url.pathname === routeHref(href) || page.url.pathname.startsWith(`${routeHref(href)}/`);
  }
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
        class:bg-surface-2={isActive(item.href)}
        class:!text-fg={isActive(item.href)}
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
