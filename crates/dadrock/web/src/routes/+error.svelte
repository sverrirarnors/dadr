<script lang="ts">
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { ArrowLeft } from 'lucide-svelte';
</script>

<div class="flex min-h-full items-center justify-center p-12">
  <div class="max-w-md text-center">
    <div class="font-mono text-5xl text-fg-subtle">{page.status}</div>
    <h1 class="mt-4 text-xl font-semibold">{page.error?.message ?? 'Something went wrong.'}</h1>
    <p class="mt-2 text-sm text-fg-muted">The page you requested could not be loaded.</p>
    <a
      href={resolve('/records')}
      class="mt-6 inline-flex h-9 items-center gap-1.5 rounded-md bg-accent px-3 text-sm text-accent-fg"
    >
      <ArrowLeft class="h-4 w-4" aria-hidden="true" />
      <span>Back to records</span>
    </a>
  </div>
</div>
