<!-- Muted explanation under a control; `warn` for a capability the running service lacks. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children, warn = false }: { children: Snippet; warn?: boolean } = $props();
</script>

<p class="hint t-small" class:warn>{@render children()}</p>

<style>
  .hint {
    margin: 0;
    color: var(--color-text-3);
  }
  .hint :global(code) {
    font-family: var(--type-mono-family);
    font-size: var(--type-mono-size);
    color: var(--color-text-2);
  }
  .warn {
    color: var(--color-warning);
  }
</style>
