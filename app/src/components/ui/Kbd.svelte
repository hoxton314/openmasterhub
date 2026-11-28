<!-- One key cap (mono 12/500). -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();
</script>

<kbd>{@render children()}</kbd>

<style>
  kbd {
    display: inline-flex;
    align-items: center;
    height: var(--size-kbd);
    padding: 0 8px;
    font: 500 var(--type-mono-size) / 1 var(--type-mono-family);
    white-space: nowrap;
    color: var(--kbd-fg);
    background: var(--kbd-bg);
    border: 1px solid var(--kbd-border);
    border-radius: var(--radius-kbd);
    box-shadow: 0 1px 0 var(--kbd-border);
  }
</style>
