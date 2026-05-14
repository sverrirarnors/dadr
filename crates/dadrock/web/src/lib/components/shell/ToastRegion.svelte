<script lang="ts">
  import { toasts, type ToastSeverity } from '$lib/stores/toast.svelte';
  import { AlertTriangle, CheckCircle2, Info, X, XCircle } from 'lucide-svelte';

  const ICONS: Record<ToastSeverity, typeof Info> = {
    info: Info,
    success: CheckCircle2,
    warning: AlertTriangle,
    error: XCircle,
  };

  const RING: Record<ToastSeverity, string> = {
    info: 'border-border',
    success: 'border-success/40',
    warning: 'border-warning/50',
    error: 'border-danger/50',
  };
</script>

<div class="pointer-events-none fixed bottom-4 right-4 z-50 flex flex-col gap-2">
  {#each toasts.list as toast (toast.id)}
    {@const Icon = ICONS[toast.severity]}
    <div
      role="status"
      class="pointer-events-auto flex max-w-sm items-start gap-2 rounded-lg border bg-bg px-3 py-2 shadow-lg {RING[
        toast.severity
      ]}"
    >
      <Icon class="mt-0.5 h-4 w-4 shrink-0 text-fg-muted" />
      <div class="flex-1 text-sm text-fg">
        <div>{toast.message}</div>
        {#if toast.detail}
          <div class="mt-0.5 text-xs text-fg-muted">{toast.detail}</div>
        {/if}
      </div>
      <button
        type="button"
        class="-m-1 p-1 text-fg-subtle hover:text-fg"
        aria-label="Dismiss"
        onclick={() => toasts.dismiss(toast.id)}
      >
        <X class="h-3.5 w-3.5" />
      </button>
    </div>
  {/each}
</div>
