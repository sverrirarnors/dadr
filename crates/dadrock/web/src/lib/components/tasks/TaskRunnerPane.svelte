<script lang="ts">
  import type { Snippet } from 'svelte';

  type Props = {
    children?: Snippet;
  };

  let { children }: Props = $props();
</script>

<section data-task-runner-pane class="mt-5">
  {@render children?.()}
</section>
