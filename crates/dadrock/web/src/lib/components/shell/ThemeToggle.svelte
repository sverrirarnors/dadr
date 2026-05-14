<script lang="ts">
  import { Moon, Monitor, Sun } from 'lucide-svelte';
  import { theme } from '$lib/stores/theme.svelte';

  const labels = {
    light: 'Light theme',
    dark: 'Dark theme',
    system: 'System theme',
  };
</script>

<button
  type="button"
  class="inline-flex h-8 w-8 items-center justify-center rounded-md text-fg-muted transition-colors hover:bg-surface-2 hover:text-fg"
  aria-label={labels[theme.current]}
  title={labels[theme.current]}
  onclick={() => theme.cycle()}
>
  {#if theme.current === 'light'}
    <Sun class="h-4 w-4" />
  {:else if theme.current === 'dark'}
    <Moon class="h-4 w-4" />
  {:else}
    <Monitor class="h-4 w-4" />
  {/if}
</button>
