<script lang="ts">
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { isPrimaryNavItemActive, primaryNavItems, secondaryNavItems } from '$lib/navigation/routes';
  import ThemeToggle from './ThemeToggle.svelte';
  let expanded = $state(false);
  let moreButton: HTMLButtonElement;
</script>

<svelte:window onkeydown={(event) => { if (event.key === 'Escape' && expanded) { expanded = false; moreButton.focus(); } }} />

<header data-app-header class="flex min-h-11 flex-wrap items-center gap-2 border-b border-border bg-bg px-2 py-1">
  <div class="text-sm font-medium text-fg">dadrock</div>
  <nav class="flex items-center gap-1" aria-label="Primary navigation">
    {#each primaryNavItems as item (item.href)}
      <a href={resolve(item.href)} aria-current={isPrimaryNavItemActive(item, page.url) ? 'page' : undefined}
        class="inline-flex h-8 items-center rounded-md px-2 text-sm text-fg-muted hover:bg-surface-2 hover:text-fg focus-visible:ring-2 focus-visible:ring-accent"
        class:bg-surface-2={isPrimaryNavItemActive(item, page.url)}>{item.label}</a>
    {/each}
    <div class="relative">
      <button bind:this={moreButton} type="button" aria-expanded={expanded} aria-controls="secondary-navigation"
        class="h-8 rounded-md px-2 text-sm text-fg-muted hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent"
        onclick={() => expanded = !expanded}>More</button>
      {#if expanded}
        <div id="secondary-navigation" class="absolute right-0 top-full z-50 min-w-40 rounded-md border border-border bg-bg p-1 shadow-lg">
          {#each secondaryNavItems as item (item.href)}
            <a href={resolve(item.href)} onclick={() => expanded = false}
              aria-current={isPrimaryNavItemActive(item, page.url) ? 'page' : undefined}
              class="block rounded px-3 py-2 text-sm hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent">{item.label}</a>
          {/each}
        </div>
      {/if}
    </div>
  </nav>
  <div class="flex-1"></div>
  <ThemeToggle />
</header>
