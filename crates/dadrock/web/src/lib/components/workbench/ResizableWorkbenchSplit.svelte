<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  import { cn } from '$lib/utils';
  import WorkbenchPage from './WorkbenchPage.svelte';

  type Props = HTMLAttributes<HTMLDivElement> & {
    splitId: string;
    handleLabel: string;
    minLeftWidth?: number;
    minRightWidth?: number;
    class?: string;
    left: Snippet;
    right: Snippet;
  };

  const DEFAULT_LEFT_WIDTH = 380;
  const HANDLE_WIDTH = 8;

  let {
    splitId,
    handleLabel,
    minLeftWidth = 288,
    minRightWidth = 420,
    class: className,
    style,
    left,
    right,
    ...rest
  }: Props = $props();

  let root = $state<HTMLDivElement | null>(null);
  let leftWidth = $state(DEFAULT_LEFT_WIDTH);
  const splitStyle = $derived(
    `--workbench-split-left-width: ${leftWidth}px;${style ? ` ${style}` : ''}`,
  );

  function startResize(event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();

    const startX = event.clientX;
    const startWidth = leftWidth;
    const maxWidth = maxLeftWidth();
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';

    const resize = (moveEvent: PointerEvent) => {
      leftWidth = clamp(
        startWidth + moveEvent.clientX - startX,
        minLeftWidth,
        maxWidth,
      );
    };
    const stop = () => {
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      window.removeEventListener('pointermove', resize);
      window.removeEventListener('pointerup', stop);
      window.removeEventListener('pointercancel', stop);
    };

    window.addEventListener('pointermove', resize);
    window.addEventListener('pointerup', stop);
    window.addEventListener('pointercancel', stop);
  }

  function resizeWithKeyboard(event: KeyboardEvent) {
    const increment = event.shiftKey ? 48 : 24;
    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      leftWidth = clamp(leftWidth - increment, minLeftWidth, maxLeftWidth());
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      leftWidth = clamp(leftWidth + increment, minLeftWidth, maxLeftWidth());
    }
  }

  function maxLeftWidth(): number {
    const rootWidth = root?.getBoundingClientRect().width ?? DEFAULT_LEFT_WIDTH;
    return Math.max(minLeftWidth, rootWidth - minRightWidth - HANDLE_WIDTH);
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }
</script>

<WorkbenchPage
  bind:element={root}
  data-resizable-workbench-split={splitId}
  layout="two-pane"
  class={cn('grid-cols-1 lg:grid-cols-[var(--workbench-split-left-width)_8px_minmax(0,1fr)]', className)}
  style={splitStyle}
  {...rest}
>
  {@render left()}

  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    data-resizable-workbench-split-handle={splitId}
    role="separator"
    aria-label={handleLabel}
    aria-orientation="vertical"
    aria-valuemin={minLeftWidth}
    aria-valuenow={Math.round(leftWidth)}
    aria-valuemax={Math.round(maxLeftWidth())}
    tabindex="0"
    class="hidden cursor-col-resize items-stretch justify-center px-1 outline-none focus-visible:ring-2 focus-visible:ring-accent lg:flex"
    onpointerdown={startResize}
    onkeydown={resizeWithKeyboard}
  >
    <span class="my-1 w-px rounded bg-border transition-colors hover:bg-accent"></span>
  </div>

  {@render right()}
</WorkbenchPage>
